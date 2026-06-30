#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FieldAlignment {
    #[default]
    Left = 0,
    Right = 1,
    Auto = 2,
    Center = 3,
}
