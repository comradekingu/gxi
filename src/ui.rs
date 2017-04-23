use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::ops::DerefMut;
use std::process::{Child, ChildStdin, Command};
use std::rc::Rc;

use cairo::Context;
use cairo::enums::FontSlant;

use gdk::{CONTROL_MASK, Cursor, DisplayManager, EventButton, EventMotion, EventKey, EventType,
    EventMask, ModifierType, SHIFT_MASK};
use gdk_sys::GdkCursorType;
use gtk;
use gtk::prelude::*;
use gtk::*;

use serde_json;
use serde_json::Value;

use xi_core_lib::rpc::Request;
use xi_core_lib::rpc::{EditCommand, TabCommand};

use error::GxiError;
use key;
use linecache::*;
use structs::*;
use util::*;
use GLOBAL;
use document::Document;

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move |$(clone!(@param $p),)+| $body
        }
    );
}

#[derive(Debug)]
pub struct XiCore<'a> {
    rpc_index: usize,
    core_stdin: ChildStdin,
    pending: HashMap<usize, Request<'a>>,
}

#[derive(Debug)]
pub struct Ui<'a> {
    xicore: XiCore<'a>,
    window: Window,
    new_button: Button,
    notebook: Notebook,
    open_file_chooser: FileChooserDialog,
    view_to_idx: HashMap<String, u32>,
    idx_to_view: HashMap<u32, String>,
    da_to_view: HashMap<Layout, String>,
    sb_to_view: HashMap<Scrollbar, String>,
    view_to_doc: HashMap<String, Document>,
}

impl XiCore<'static> {
    /// Build and send a JSON RPC request, returning the associated request ID to pair it with
    /// the response
    fn request(&mut self, method: &str, params: Value) -> usize {
        self.rpc_index += 1;
        let message = json!({
            "id": self.rpc_index,
            "method": method,
            "params": params,
        });
        self.send(&message);
        self.rpc_index
    }

    fn notify(&mut self, method: &str, params: Value) {
        let message = json!({
            "method": method,
            "params": params,
        });
        self.send(&message);
    }

    fn edit(&mut self, method: &str, view_id: &str, params: Value) {
        self.notify("edit", json!({
            "method": method,
            "view_id": view_id,
            "params": params,
        }));
    }

    /// Serialize JSON object and send it to the server
    fn send(&mut self, message: &Value) {
        // debug!(">>> {:?}", message);
        let mut str_msg = serde_json::ser::to_string(&message).unwrap();
        debug!("<<< {}", str_msg);
        str_msg.push('\n');
        self.core_stdin.write_all(str_msg.as_bytes()).unwrap();
    }

    fn save(&mut self, view_id: &str, file_path: &str) {
        self.notify("save", json!({
            "view_id": view_id,
            "file_path": file_path,
        }));
    }

    fn delete_forward(&mut self, view_id: &str) {
        self.edit("delete_forward", view_id, json!([]));
    }
    fn delete_backward(&mut self, view_id: &str) {
        self.edit("delete_backward", view_id, json!([]));
    }
    fn insert_newline(&mut self, view_id: &str) {
        self.edit("insert_newline", view_id, json!([]));
    }
    fn insert_tab(&mut self, view_id: &str) {
        self.edit("insert_tab", view_id, json!([]));
    }
    fn move_up(&mut self, view_id: &str) {
        self.edit("move_up", view_id, json!([]));
    }
    fn move_down(&mut self, view_id: &str) {
        self.edit("move_down", view_id, json!([]));
    }
    fn move_left(&mut self, view_id: &str) {
        self.edit("move_left", view_id, json!([]));
    }
    fn move_right(&mut self, view_id: &str) {
        self.edit("move_right", view_id, json!([]));
    }
    fn move_up_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_up_and_modify_selection", view_id, json!([]));
    }
    fn move_down_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_down_and_modify_selection", view_id, json!([]));
    }
    fn move_left_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_left_and_modify_selection", view_id, json!([]));
    }
    fn move_right_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_right_and_modify_selection", view_id, json!([]));
    }
    fn move_word_left(&mut self, view_id: &str) {
        self.edit("move_word_left", view_id, json!([]));
    }
    fn move_word_right(&mut self, view_id: &str) {
        self.edit("move_word_right", view_id, json!([]));
    }
    fn move_word_left_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_word_left_and_modify_selection", view_id, json!([]));
    }
    fn move_word_right_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_word_right_and_modify_selection", view_id, json!([]));
    }
    fn move_to_left_end_of_line(&mut self, view_id: &str) {
        self.edit("move_to_left_end_of_line", view_id, json!([]));
    }
    fn move_to_right_end_of_line(&mut self, view_id: &str) {
        self.edit("move_to_right_end_of_line", view_id, json!([]));
    }
    fn move_to_left_end_of_line_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_to_left_end_of_line_and_modify_selection", view_id, json!([]));
    }
    fn move_to_right_end_of_line_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_to_right_end_of_line_and_modify_selection", view_id, json!([]));
    }
    fn move_to_beginning_of_document(&mut self, view_id: &str) {
        self.edit("move_to_beginning_of_document", view_id, json!([]));
    }
    fn move_to_beginning_of_document_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_to_beginning_of_document_and_modify_selection", view_id, json!([]));
    }
    fn move_to_end_of_document(&mut self, view_id: &str) {
        self.edit("move_to_end_of_document", view_id, json!([]));
    }
    fn move_to_end_of_document_and_modify_selection(&mut self, view_id: &str) {
        self.edit("move_to_end_of_document_and_modify_selection", view_id, json!([]));
    }
    fn page_up(&mut self, view_id: &str) {
        self.edit("page_up", view_id, json!([]));
    }
    fn page_down(&mut self, view_id: &str) {
        self.edit("page_down", view_id, json!([]));
    }
    fn page_up_and_modify_selection(&mut self, view_id: &str) {
        self.edit("page_up_and_modify_selection", view_id, json!([]));
    }
    fn page_down_and_modify_selection(&mut self, view_id: &str) {
        self.edit("page_down_and_modify_selection", view_id, json!([]));
    }
    fn select_all(&mut self, view_id: &str) {
        self.edit("select_all", view_id, json!([]));
    }
    fn transpose(&mut self, view_id: &str) {
        self.edit("transpose", view_id, json!([]));
    }
    fn undo(&mut self, view_id: &str) {
        self.edit("undo", view_id, json!([]));
    }
    fn redo(&mut self, view_id: &str) {
        self.edit("redo", view_id, json!([]));
    }
    fn cut(&mut self, view_id: &str) {
        self.edit("cut", view_id, json!([]));
    }
    fn copy(&mut self, view_id: &str) {
        self.edit("copy", view_id, json!([]));
    }
}

impl Ui<'static> {
    pub fn new(core_stdin: ChildStdin) -> Rc<RefCell<Ui<'static>>> {
        let builder = Builder::new_from_file("resources/gxi.ui");
        let window: Window = builder.get_object("appwindow").unwrap();
        let notebook: Notebook = builder.get_object("notebook").unwrap();
        let new_button: Button = builder.get_object("new_button").unwrap();
        let open_button: Button = builder.get_object("open_button").unwrap();
        let save_button: Button = builder.get_object("save_button").unwrap();
        let open_file_chooser: FileChooserDialog = builder.get_object("open_file_chooser").unwrap();
        let xi_core = XiCore{
            rpc_index: 0,
            core_stdin: core_stdin,
            pending: HashMap::new(),
        };

        let ui = Rc::new(RefCell::new(Ui {
            xicore: xi_core,
            window: window.clone(),
            new_button: new_button.clone(),
            notebook: notebook.clone(),
            open_file_chooser: open_file_chooser.clone(),
            view_to_idx: HashMap::new(),
            idx_to_view: HashMap::new(),
            da_to_view: HashMap::new(),
            sb_to_view: HashMap::new(),
            view_to_doc: HashMap::new(),
        }));

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        new_button.connect_clicked(clone!(ui => move |_| {
            ui.borrow_mut().request_new_view();
        }));
        open_button.connect_clicked(handle_open_button);
        save_button.connect_clicked(handle_save_button);

        ui
    }

    /// Called when xi-core gives us a new line
    pub fn handle_line(&mut self, line: &str) -> Result<(), GxiError> {
        debug!(">>> {}", line);
        let json: Value = serde_json::from_str(line)?;
        //debug!("json: {:?}", json);
        let is_method = json.as_object().map_or(false, |dict|
            dict.contains_key("method"));
        if is_method {
            self.handle_method(json)?;
        } else {
            self.handle_response(json)?;
        }
        //match()
        Ok(())
    }

    pub fn handle_method(&mut self, json: Value) -> Result<(), GxiError> {
        let method = json.as_object().unwrap().get("method").unwrap().as_str().unwrap();
        let params = json.as_object().unwrap().get("params").unwrap();
        match method {
            "scroll_to" => {
                params
                .as_object()
                .ok_or_else(|| GxiError::MalformedMethodParams(method.to_string(), params.clone()))
                .and_then(|dict| {
                    debug!("dict={:?}", dict);
                    if let (Some(view_id), Some(line), Some(col)) =
                        (dict_get_string(dict, "view_id"),
                        dict_get_u64(dict, "line"),
                        dict_get_u64(dict, "col")) {
                            self.handle_scroll_to(view_id, line, col)
                    } else {Err(GxiError::MalformedMethodParams(method.to_string(), params.clone()))}
                })
            }
            "update" => {
                let p: UpdateParams = serde_json::from_value(params.clone())?;
                self.handle_update(&p.view_id, &p.update.ops)
            }
            _ => Err(GxiError::UnknownMethod(method.to_string()))
        }
        //Ok(())
    }

    pub fn handle_response(&mut self, mut response: Value) -> Result<(), GxiError> {
        let mut dict = response.as_object_mut().unwrap();
        let id = dict.get("id").and_then(Value::as_u64);
        if id.is_none() {
            return Err(GxiError::Custom("id missing from response, or is not u64".to_string()));
        }
        let id = id.unwrap() as usize;
        let result = dict.remove("result");
        let error = dict.remove("error");
        //let req = self.pending.remove(id);
        let req = match self.xicore.pending.remove(&id) {
            None => {return Err(GxiError::Custom(format!("Unexpected id: {}", id)));}
            Some(req) => req,
        };
        match req {
            Request::TabCommand{ tab_command } => match tab_command {
                TabCommand::NewTab => {
                    //if let Some(tab_name) = dict_get_string()
                    result
                    .ok_or_else(|| GxiError::Custom("No result on new tab".to_string()))
                    //.as_str()
                    .and_then(|result| {
                        if let Some(view_id) = result.as_str() {
                            self.response_new_tab(view_id)
                        } else {Err(GxiError::Custom("Unexpected result type on new view".to_string()))}
                    })

                },
                TabCommand::DeleteTab{ tab_name } => self.response_delete_view(tab_name),
                _ => Err(GxiError::Custom("Unexpected result".to_string()))

                // TabCommand::Edit{tab_name, edit_command} => match edit_command {
                //         EditCommand::RenderLines { first_line, last_line } => {},
                //         EditCommand::Key { chars, flags } => {},
                //         EditCommand::Insert { chars } => {},
                //         EditCommand::DeleteForward => {},
                //         EditCommand::DeleteBackward => {},
                //         EditCommand::DeleteToEndOfParagraph => {},
                //         EditCommand::DeleteToBeginningOfLine => {},
                //         EditCommand::InsertNewline => {},
                //         EditCommand::InsertTab => {},
                //         EditCommand::MoveUp => {},
                //         EditCommand::MoveUpAndModifySelection => {},
                //         EditCommand::MoveDown => {},
                //         EditCommand::MoveDownAndModifySelection => {},
                //         EditCommand::MoveLeft => {},
                //         EditCommand::MoveLeftAndModifySelection => {},
                //         EditCommand::MoveRight => {},
                //         EditCommand::MoveRightAndModifySelection => {},
                //         // EditCommand::MoveWordLeft => {},
                //         // EditCommand::MoveWordLeftAndModifySelection => {},
                //         // EditCommand::MoveWordRight => {},
                //         // EditCommand::MoveWordRightAndModifySelection => {},
                //         EditCommand::MoveToBeginningOfParagraph => {},
                //         EditCommand::MoveToEndOfParagraph => {},
                //         EditCommand::MoveToLeftEndOfLine => {},
                //         EditCommand::MoveToLeftEndOfLineAndModifySelection => {},
                //         EditCommand::MoveToRightEndOfLine => {},
                //         EditCommand::MoveToRightEndOfLineAndModifySelection => {},
                //         EditCommand::MoveToBeginningOfDocument => {},
                //         EditCommand::MoveToBeginningOfDocumentAndModifySelection => {},
                //         EditCommand::MoveToEndOfDocument => {},
                //         EditCommand::MoveToEndOfDocumentAndModifySelection => {},
                //         EditCommand::ScrollPageUp => {},
                //         EditCommand::PageUpAndModifySelection => {},
                //         EditCommand::ScrollPageDown => {},
                //         EditCommand::PageDownAndModifySelection => {},
                //         // EditCommand::SelectAll => {},
                //         EditCommand::Open { file_path } => {},
                //         EditCommand::Save { file_path } => {},
                //         EditCommand::Scroll { first, last } => {},
                //         // EditCommand::RequestLines { first, last } => {},
                //         EditCommand::Yank => {},
                //         EditCommand::Transpose => {},
                //         EditCommand::Click { line, column, flags, click_count } => {},
                //         EditCommand::Drag { line, column, flags } => {},
                //         EditCommand::Undo => {},
                //         EditCommand::Redo => {},
                //         EditCommand::Cut => {},
                //         EditCommand::Copy => {},
                //         EditCommand::DebugRewrap => {},
                //         EditCommand::DebugTestFgSpans => {},
                //         EditCommand::DebugRunPlugin => {},
                    // },
            }
        }
    }

    pub fn handle_update(&mut self, view_id: &str, ops: &Vec<UpdateOp>) -> Result<(), GxiError> {
        debug!("update: {:?}", ops);
        let mut doc = self.view_to_doc.get_mut(view_id).unwrap(); //FIXME error handling

        doc.handle_update(ops);

        let mut new_invalid_before = 0;
        let new_lines: Vec<Option<Line>> = Vec::new();
        let mut new_invalid_after = 0;

        for op in ops {
            // let op_type = op.op;
            let mut idx = 0;
            let mut n = op.n;
            // let mut old_ix = 0;
            // match op_type.as_ref() {
            //     "invalidate" => {
            //         if new_lines.len() == 0 {
            //             new_invalid_before += n;
            //         } else {
            //             new_invalid_after += n;
            //         }
            //     },
            //     "ins" => {
            //         for _ in 0..new_invalid_after {
            //             new_lines.push(None);
            //         }
            //         new_invalid_after = 0;
            //         let json_lines = op.lines.unwrap_or_else(Vec::new);
            //         for json_line in json_lines {
            //             new_lines.push(Some(Line{
            //                 cursor: json_line.cursor.unwrap_or_else(Vec::new),
            //                 text: json_line.text,
            //             }));
            //         }
            //     },
            //     "copy" | "update" => {
            //         let n_remaining = n;
            //         if old_ix < n_invalid_before {
            //
            //         }
            //     },
            //     "skip" => {
            //
            //     },
            //     _ => {
            //
            //     },
            // }



            // for update_line in op.lines.iter().flat_map(|l| l.iter()) {
            //     let mut cursor: Vec<usize> = Vec::new();
            //     if let Some(ref ul_cursor) = update_line.cursor {
            //         cursor.append(&mut ul_cursor.clone());
            //     }
            //     let line = Line{
            //         cursor: cursor,
            //         text: update_line.text.clone(),
            //     };
            //     doc.line_cache.insert(idx as u64, line);
            //     doc.drawing_area.queue_draw();
            //     idx += 1;
            // }
        }
        Ok(())
    }

    pub fn handle_scroll_to(&mut self, view_id: &str, line: u64, col: u64) -> Result<(), GxiError> {
        debug!("scroll_to {} {} {}", view_id, line, col);
        if let Some(idx) = self.view_to_idx.get(view_id) {
            self.notebook.set_current_page(Some(*idx));
        }
        let mut doc = self.view_to_doc.get_mut(view_id).unwrap();
        doc.scroll_to(line, col);
        Ok(())
    }

    pub fn show_all(&self) {
        self.window.show_all();
    }

    pub fn request_new_view(&mut self) {
        let req = Request::TabCommand{tab_command: TabCommand::NewTab};
        let id = self.xicore.request("new_view", json!({}));
        self.xicore.pending.insert(id, req);
    }

    pub fn request_new_view_file(&mut self, path: &str) {
        let req = Request::TabCommand{tab_command: TabCommand::NewTab};
        let id = self.xicore.request("new_view", json!({"file_path": path}));
        self.xicore.pending.insert(id, req);
    }

    pub fn request_delete_view(&mut self, view_id: &str) -> Result<(), GxiError> {
        Ok(())
    }

    pub fn response_delete_view(&mut self, view_id: &str) -> Result<(), GxiError> {
        Ok(())
    }

    pub fn response_new_tab(&mut self, view_id: &str) -> Result<(), GxiError> {
        let adj = Adjustment::new(0.0, 0.0, 3.0, 1.0, 2.0, 1.0);
        let scrolled_window = ScrolledWindow::new(None, None);
        let drawing_area = Layout::new(None, Some(&adj));
        //let ui = self.clone();
        debug!("events={:?}", drawing_area.get_events());
        //drawing_area.set_events(EventMask::all().bits() as i32);
        drawing_area.set_events(::gdk::BUTTON_PRESS_MASK.bits() as i32 | ::gdk::BUTTON_MOTION_MASK.bits() as i32);
        debug!("events={:?}", drawing_area.get_events());
        drawing_area.set_can_focus(true);
        drawing_area.connect_button_press_event(handle_button_press);
        drawing_area.connect_key_press_event(handle_key_press_event);
        // drawing_area.connect_key_release_event(|w,ek| {
        //     debug!("key release {:?}", ek);
        //     Inhibit(false)
        // });
        drawing_area.connect_draw(handle_draw);

        drawing_area.connect_size_allocate(|_,alloc| {
            debug!("Size changed to w={} h={}", alloc.width, alloc.height);
        });

        drawing_area.connect_realize(|w|{
            // Set the text cursor
            DisplayManager::get().get_default_display()
                .map(|disp| {
                    let cur = Cursor::new_for_display(&disp, GdkCursorType::Xterm);
                    w.get_window().map(|win| win.set_cursor(&cur));
            });
            w.grab_focus();
        });

        drawing_area.connect_drag_motion(|w,dc,a,b,c| {
            debug!("dc={:?} a={}, b={}, c={}", dc, a, b, c);
            false
        });
        drawing_area.connect_motion_notify_event(handle_drag);
        // drawing_area.connect_scroll_event(|w,e|{
        //     debug!("scroll event {:?} {:?}", w, e);
        //     Inhibit(false)
        // });

        // scrolled_window.connect_scroll_child(|w,a,b| {
        //     debug!("scrolled_window.connect_scroll_child {:?} {:?}", a, b);
        //     true
        // });
        // scrolled_window.connect_draw(|w,cr| {
        //     debug!("connect_draw scrolled_window");
        //     Inhibit(false)
        // });


        self.da_to_view.insert(drawing_area.clone(), view_id.to_owned());
        //self.sb_to_view.insert(scrollbar.clone(), view_id.to_owned());
        self.view_to_doc.insert(view_id.to_owned(), Document::new(drawing_area.clone()));
        scrolled_window.add(&drawing_area);
        let label = Label::new("Untitled");
        let view_label: Option<&Label> = Some(&label);
        let idx = self.notebook.insert_page(&scrolled_window, view_label, Some(0xffffffffu32));
        self.view_to_idx.insert(view_id.to_string(), idx);
        self.idx_to_view.insert(idx, view_id.to_string());
        self.notebook.show_all();

        // self.notify("edit", json!({"method": "scroll",
        //     "view_id": view_id,
        //     "params": [0, 30],
        // }));

        //self.notify("scroll", json!([0, 30]));
        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////////
// Gtk Handler Functions
///////////////////////////////////////////////////////////////////////////////

// NSAlphaShiftKeyMask = 1 << 16,
// NSShiftKeyMask      = 1 << 17,
// NSControlKeyMask    = 1 << 18,
// NSAlternateKeyMask  = 1 << 19,
// NSCommandKeyMask    = 1 << 20,
// NSNumericPadKeyMask = 1 << 21,
// NSHelpKeyMask       = 1 << 22,
// NSFunctionKeyMask   = 1 << 23,
// NSDeviceIndependentModifierFlagsMask = 0xffff0000U

const XI_SHIFT_KEY_MASK:u32 = 1 << 1;
const XI_CONTROL_KEY_MASK:u32 = 1 << 2;
const XI_ALT_KEY_MASK:u32 = 1 << 3;

fn convert_gtk_modifier(mt: ModifierType) -> u32 {
    let mut ret = 0;
    if mt.contains(SHIFT_MASK) { ret |= XI_SHIFT_KEY_MASK; }
    if mt.contains(CONTROL_MASK) { ret |= XI_CONTROL_KEY_MASK; }
    ret
}

fn convert_eb_to_xi_click(eb: &EventButton) -> u32 {
    match eb.get_event_type() {
        EventType::ButtonPress => 1,
        EventType::DoubleButtonPress => 2,
        EventType::TripleButtonPress => 3,
        _ => 0,
    }
}

pub fn handle_button_press(w: &Layout, eb: &EventButton) -> Inhibit {
    w.grab_focus();
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui_refmut = ui.borrow_mut();
        let mut ui = ui_refmut.deref_mut();
        let view_id = ui.da_to_view.get(w).unwrap().clone();

        let doc = ui.view_to_doc.get_mut(&view_id).unwrap();
        let (x,y) = eb.get_position();
        let (col, line) = doc.pos_to_cell(x, y);
        ui.xicore.notify("edit", json!({"method": "click",
            "view_id": view_id,
            "params": [line, col,
                convert_gtk_modifier(eb.get_state()),
                convert_eb_to_xi_click(eb)
            ],
        }));
    });
    Inhibit(false)
}

pub fn handle_drag(w: &Layout, em: &EventMotion) -> Inhibit {
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui_refmut = ui.borrow_mut();
        let mut ui = ui_refmut.deref_mut();
        let view_id = ui.da_to_view.get(w).unwrap().clone();

        let doc = ui.view_to_doc.get_mut(&view_id).unwrap();
        let (x,y) = em.get_position();
        let (col, line) = doc.pos_to_cell(x, y);
        ui.xicore.notify("edit", json!({"method": "drag",
            "view_id": view_id,
            "params": [line, col, convert_gtk_modifier(em.get_state())],
        }));
    });
    Inhibit(false)
}

pub fn handle_open_button(open_button: &Button) {
    // let mut fcd: Option<FileChooserDialog> = None;
    // GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
    //     let mut ui_refmut = ui.borrow_mut();
    //     let ui = ui_refmut.deref_mut();
    //     fcd = Some(ui.open_file_chooser.clone());
    // });
    // if let Some(fcd) = fcd {
    //     let response = fcd.run();
    //     debug!("open response={}", response);
    // }

    let mut main_window: Option<Window> = None;
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui_refmut = ui.borrow_mut();
        let ui = ui_refmut.deref_mut();
        main_window = Some(ui.window.clone());
    });
    let fcd = FileChooserDialog::new::<FileChooserDialog>(None, None, FileChooserAction::Open);
    if let Some(main_window) = main_window {
        fcd.set_transient_for(Some(&main_window));
    }
    fcd.add_button("Open", 33);
    fcd.set_default_response(33);
    fcd.set_select_multiple(true);
    let response = fcd.run();
    debug!("open response = {}", response);
    if response == 33 {
        for file in fcd.get_filenames() {
            GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
                debug!("opening {:?}", file);
                let mut ui_refmut = ui.borrow_mut();
                let ui = ui_refmut.deref_mut();
                ui.request_new_view_file(&file.to_string_lossy());
            });
        }
    }
    fcd.destroy();
}

pub fn handle_save_button(open_button: &Button) {
    let mut main_window: Option<Window> = None;
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui_refmut = ui.borrow_mut();
        let ui = ui_refmut.deref_mut();
        main_window = Some(ui.window.clone());
    });
    let fcd = FileChooserDialog::new::<FileChooserDialog>(None, None, FileChooserAction::Save);
    if let Some(main_window) = main_window {
        fcd.set_transient_for(Some(&main_window));
    }
    fcd.add_button("Save", 33);
    fcd.set_default_response(33);
    let response = fcd.run();
    debug!("save response = {}", response);
    if response == 33 {
        for file in fcd.get_filenames() {
            GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
                debug!("saving {:?}", file);
                let mut ui_refmut = ui.borrow_mut();
                let ui = ui_refmut.deref_mut();
                if let Some(idx) = ui.notebook.get_current_page() {
                    if let Some(view_id) = ui.idx_to_view.get(&idx as &u32) {
                        ui.xicore.save(view_id, &file.to_string_lossy());
                    }
                }
            });
        }
    }
    fcd.destroy();
}

fn handle_draw(w: &Layout, cr: &Context) -> Inhibit {
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui_refmut = ui.borrow_mut();
        let mut ui = ui_refmut.deref_mut();
        let view_id = ui.da_to_view.get(w).unwrap().clone();

        // let missing = ui.view_to_doc.get_mut(&view_id).unwrap().line_cache.get_missing(0, 1);
        // debug!("MISSING={:?}", missing);
        // for run in missing {
        //     ui.notify("edit", json!({"method": "request_lines",
        //         "view_id": view_id,
        //         "params": [run.0, run.1],
        //     }));
        // }

        let doc = ui.view_to_doc.get_mut(&view_id).unwrap();
        let (first_line, last_line, missing) = {
            doc.handle_draw(cr)
        };
        // debug!("MISSING={:?}", missing);
        // for run in missing {
        //     ui.notify("edit", json!({"method": "request_lines",
        //         "view_id": view_id,
        //         "params": [run.0, run.1],
        //     }));
        // }
        let xicore = &mut ui.xicore;
        if (first_line, last_line) != (doc.first_line, doc.last_line) {
            {
                doc.first_line = first_line;
                doc.last_line = last_line;
            }
            debug!("first,last={},{}", first_line, last_line);
            xicore.notify("edit", json!({"method": "scroll",
                "view_id": view_id,
                "params": [first_line, last_line],
            }));
        }
    });
    Inhibit(true)
}

fn handle_key_press_event(w: &Layout, ek: &EventKey) -> Inhibit {
    debug!("key press {:?}", ek);
    debug!("key press keyval={:?}, state={:?}, length={:?} group={:?} uc={:?}",
        ek.get_keyval(), ek.get_state(), ek.get_length(), ek.get_group(),
        ::gdk::keyval_to_unicode(ek.get_keyval())
    );
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui = ui.borrow_mut();
        let view_id = ui.da_to_view.get(&w.clone()).unwrap().clone();
        let ch = ::gdk::keyval_to_unicode(ek.get_keyval());

        match ek.get_keyval() {
            key::DEL if ek.get_state().is_empty() => ui.xicore.delete_forward(&view_id),
            key::BACKSPACE if ek.get_state().is_empty() => ui.xicore.delete_backward(&view_id),
            key::ENTER | key::ENTER_PAD if ek.get_state().is_empty() => {
                ui.xicore.insert_newline(&view_id);
            },
            key::TAB if ek.get_state().is_empty() => ui.xicore.insert_tab(&view_id),
            key::ARROW_UP if ek.get_state().is_empty() => ui.xicore.move_up(&view_id),
            key::ARROW_DOWN if ek.get_state().is_empty() => ui.xicore.move_down(&view_id),
            key::ARROW_LEFT if ek.get_state().is_empty() => ui.xicore.move_left(&view_id),
            key::ARROW_RIGHT if ek.get_state().is_empty() => ui.xicore.move_right(&view_id),
            key::ARROW_UP if ek.get_state() == SHIFT_MASK => {
                ui.xicore.move_up_and_modify_selection(&view_id);
            },
            key::ARROW_DOWN if ek.get_state() == SHIFT_MASK => {
                ui.xicore.move_down_and_modify_selection(&view_id);
            },
            key::ARROW_LEFT if ek.get_state() == SHIFT_MASK => {
                ui.xicore.move_left_and_modify_selection(&view_id);
            },
            key::ARROW_RIGHT if ek.get_state() == SHIFT_MASK => {
                ui.xicore.move_right_and_modify_selection(&view_id);
            },
            key::ARROW_LEFT if ek.get_state() == CONTROL_MASK => {
                ui.xicore.move_word_left(&view_id);
            },
            key::ARROW_RIGHT if ek.get_state() == CONTROL_MASK => {
                ui.xicore.move_word_right(&view_id);
            },
            key::ARROW_LEFT if ek.get_state() == CONTROL_MASK | SHIFT_MASK => {
                ui.xicore.move_word_left_and_modify_selection(&view_id);
            },
            key::ARROW_RIGHT if ek.get_state() == CONTROL_MASK | SHIFT_MASK => {
                ui.xicore.move_word_right_and_modify_selection(&view_id);
            },
            key::HOME if ek.get_state().is_empty() => {
                ui.xicore.move_to_left_end_of_line(&view_id);
            }
            key::END if ek.get_state().is_empty() => {
                ui.xicore.move_to_right_end_of_line(&view_id);
            }
            key::HOME if ek.get_state() == SHIFT_MASK => {
                ui.xicore.move_to_left_end_of_line_and_modify_selection(&view_id);
            }
            key::END if ek.get_state() == SHIFT_MASK => {
                ui.xicore.move_to_right_end_of_line_and_modify_selection(&view_id);
            }
            key::PGUP if ek.get_state().is_empty() => {
                ui.xicore.page_up(&view_id);
            }
            key::PGDN if ek.get_state().is_empty() => {
                ui.xicore.page_down(&view_id);
            }
            key::PGUP if ek.get_state() == SHIFT_MASK => {
                ui.xicore.page_up_and_modify_selection(&view_id);
            }
            key::PGDN if ek.get_state() == SHIFT_MASK => {
                ui.xicore.page_down_and_modify_selection(&view_id);
            }
            _ => {
                if let Some(ch) = ch {
                    match ch {
                        'a' if ek.get_state() == CONTROL_MASK => {
                            ui.xicore.select_all(&view_id);
                        },
                        'c' if ek.get_state() == CONTROL_MASK => {
                            ui.xicore.copy(&view_id);
                        },
                        't' if ek.get_state() == CONTROL_MASK => {
                            ui.request_new_view();
                        },
                        'x' if ek.get_state() == CONTROL_MASK => {
                            ui.xicore.cut(&view_id);
                        },
                        'z' if ek.get_state() == CONTROL_MASK => {
                            ui.xicore.undo(&view_id);
                        },
                        'Z' if ek.get_state() == CONTROL_MASK | SHIFT_MASK => {
                            ui.xicore.redo(&view_id);
                        },
                        c if (ek.get_state().is_empty() || ek.get_state() == SHIFT_MASK)
                            && c >= '\u{0020}' => {
                            ui.xicore.notify("edit", json!({"method": "insert",
                                "view_id": view_id,
                                "params": {"chars":c},
                            }));
                        }
                        _ => {},
                    }
                }
            },
        };
    });
    Inhibit(true)
}
