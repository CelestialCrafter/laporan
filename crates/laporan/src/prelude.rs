//! Common imports for `laporan`

pub use core::{error::Error as StdError, result::Result as StdResult};

pub use crate::{
    BoxDynError, ErasedReport, ErasedResult, IntoReport, Report, ReportExt, Result, ResultReportExt,
};
