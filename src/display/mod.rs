mod billing;
mod daily;
mod helpers;
mod json;
mod model_breakdown;
mod monthly;
mod session;
mod summary;
mod weekly;

// Re-export all public functions so `use crate::display::X` continues to work
pub use billing::display_billing_blocks_responsive;
pub use daily::{
    display_daily_report_compact, display_daily_report_enhanced, display_daily_report_responsive,
    display_daily_report_table,
};
pub use helpers::{print_error, print_info, print_warning};
pub use json::display_report_json;
pub use model_breakdown::display_model_breakdown_report;
pub use monthly::{display_monthly_report_enhanced, display_monthly_report_table};
pub use session::{
    display_session_report_enhanced, display_session_report_responsive,
    display_session_report_table,
};
pub use weekly::{display_weekly_report_enhanced, display_weekly_report_table};
