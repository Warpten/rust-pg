pub trait Column {
    fn read(&self) -> Option<&[u8]>;
}

pub trait ColumnSpec {
    fn name(&self) -> &str;
}
