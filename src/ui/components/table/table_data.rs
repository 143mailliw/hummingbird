use std::{fmt::Debug, hash::Hash, sync::Arc};

use gpui::{App, ElementId, SharedString};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;

pub trait Column: Clone + Copy + Debug + Hash + PartialEq + Eq {
    /// Retrieves the friendly name text of the column.
    fn get_column_name(&self) -> &'static str;
}

#[derive(Copy, Clone)]
pub struct TableSort<C>
where
    C: Column,
{
    pub column: C,
    pub ascending: bool,
}

/// The TableData trait defines the interface for retrieving, sorting, and listing data for a table.
/// Implementing this trait allows a table to display data in a structured manner.
pub trait TableData<C>: Sized
where
    C: Column,
{
    type Identifier: Clone + Debug;

    /// Retrieves the name of the table.
    fn get_table_name() -> &'static str;

    /// Retrieves the rows of the table. The rows are returned as a vector of identifiers, which
    /// can be used to retrieve the full row data. The sort parameter can be used to specify the
    /// sorting order of the rows.
    fn get_rows(cx: &mut App, sort: Option<TableSort<C>>) -> anyhow::Result<Vec<Self::Identifier>>;

    /// Retrieves a specific row of the table. The row is returned as an Arc to the table data,
    /// which can be used to retrieve the row data as SharedStrings. The id parameter is used to
    /// identify the row to retrieve.
    fn get_row(cx: &mut App, id: Self::Identifier) -> anyhow::Result<Option<Arc<Self>>>;

    /// Retrieves a column from the row.
    fn get_column(&self, cx: &mut App, column: C) -> Option<SharedString>;

    /// Returns true if the rows may contain images. This is used during the layout phase to
    /// determine if placeholder covers and the header section should be displayed.
    fn has_images() -> bool;

    /// Retrieves the associated image for the row.
    fn get_image_path(&self) -> Option<SharedString>;

    /// Retrieves the default column widths for the table.
    fn default_columns() -> IndexMap<C, f32, FxBuildHasher>;

    /// Returns a boolean indicating whether or not a given column should be displayed using a
    /// monospaced font.
    ///
    /// This should be true for columns that contain mostly numbers, like a date or time.
    fn column_monospace(column: C) -> bool;

    /// Retrieves a unique element id for the row. This is different from the row id, as it is
    /// used to identify the row in GPUI.
    fn get_element_id(&self) -> impl Into<ElementId>;

    /// Retrieves the table ID for the row.
    fn get_table_id(&self) -> Self::Identifier;
}
