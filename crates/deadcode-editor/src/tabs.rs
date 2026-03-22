#[derive(Debug, Clone)]
pub struct TabState {
    pub script_id: String,
    pub name: String,
    pub is_modified: bool,
}

#[derive(Default)]
pub struct EditorWindowState {
    pub tabs: Vec<TabState>,
    pub active_tab_index: Option<usize>,
}

impl EditorWindowState {
    pub fn open_tab(&mut self, script_id: String, name: String) {
        // Don't open duplicate tabs
        if let Some(idx) = self.tabs.iter().position(|t| t.script_id == script_id) {
            self.active_tab_index = Some(idx);
            return;
        }
        self.tabs.push(TabState {
            script_id,
            name,
            is_modified: false,
        });
        self.active_tab_index = Some(self.tabs.len() - 1);
    }

    pub fn close_tab(&mut self, script_id: &str) {
        if let Some(idx) = self.tabs.iter().position(|t| t.script_id == script_id) {
            self.tabs.remove(idx);
            if self.tabs.is_empty() {
                self.active_tab_index = None;
            } else if let Some(active) = self.active_tab_index
                && active >= self.tabs.len() {
                    self.active_tab_index = Some(self.tabs.len() - 1);
                }
        }
    }

    pub fn set_modified(&mut self, script_id: &str, modified: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.script_id == script_id) {
            tab.is_modified = modified;
        }
    }
}
