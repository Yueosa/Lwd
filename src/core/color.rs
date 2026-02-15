#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<[u8; 4]> for ColorRgba {
    fn from(value: [u8; 4]) -> Self {
        Self {
            r: value[0],
            g: value[1],
            b: value[2],
            a: value[3],
        }
    }
}

impl ColorRgba {
    pub fn as_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}
