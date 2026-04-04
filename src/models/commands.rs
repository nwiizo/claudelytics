// Command palette actions
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CommandAction {
    SwitchTab(usize),
    #[allow(dead_code)]
    ToggleSort(String),
    #[allow(dead_code)]
    SetFilter(String),
    #[allow(dead_code)]
    ExportData(String),
    #[allow(dead_code)]
    BookmarkSession(String),
    #[allow(dead_code)]
    CompareSelected,
    #[allow(dead_code)]
    ShowBenchmark,
    RefreshData,
    #[allow(dead_code)]
    OpenSessionDetail(String),
    ShowHelp,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,
    pub action: CommandAction,
    pub category: String,
}
