#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl BColor {
    pub const INDIANRED: BColor = BColor::new(205, 92, 92, 255);
    pub const LIGHTCORAL: BColor = BColor::new(240, 128, 128, 255);
    pub const SALMON: BColor = BColor::new(250, 128, 114, 255);
    pub const DARKSALMON: BColor = BColor::new(233, 150, 122, 255);
    pub const LIGHTSALMON: BColor = BColor::new(255, 160, 122, 255);
    pub const CRIMSON: BColor = BColor::new(220, 20, 60, 255);
    pub const RED: BColor = BColor::new(255, 0, 0, 255);
    pub const FIREBRICK: BColor = BColor::new(178, 34, 34, 255);
    pub const DARKRED: BColor = BColor::new(139, 0, 0, 255);
    pub const PINK: BColor = BColor::new(255, 192, 203, 255);
    pub const LIGHTPINK: BColor = BColor::new(255, 182, 193, 255);
    pub const HOTPINK: BColor = BColor::new(255, 105, 180, 255);
    pub const DEEPPINK: BColor = BColor::new(255, 20, 147, 255);
    pub const MEDIUMVIOLETRED: BColor = BColor::new(199, 21, 133, 255);
    pub const PALEVIOLETRED: BColor = BColor::new(219, 112, 147, 255);
    pub const CORAL: BColor = BColor::new(255, 127, 80, 255);
    pub const TOMATO: BColor = BColor::new(255, 99, 71, 255);
    pub const ORANGERED: BColor = BColor::new(255, 69, 0, 255);
    pub const DARKORANGE: BColor = BColor::new(255, 140, 0, 255);
    pub const ORANGE: BColor = BColor::new(255, 165, 0, 255);
    pub const GOLD: BColor = BColor::new(255, 215, 0, 255);
    pub const YELLOW: BColor = BColor::new(255, 255, 0, 255);
    pub const LIGHTYELLOW: BColor = BColor::new(255, 255, 224, 255);
    pub const LEMONCHIFFON: BColor = BColor::new(255, 250, 205, 255);
    pub const LIGHTGOLDENRODYELLOW: BColor = BColor::new(250, 250, 210, 255);
    pub const PAPAYAWHIP: BColor = BColor::new(255, 239, 213, 255);
    pub const MOCCASIN: BColor = BColor::new(255, 228, 181, 255);
    pub const PEACHPUFF: BColor = BColor::new(255, 218, 185, 255);
    pub const PALEGOLDENROD: BColor = BColor::new(238, 232, 170, 255);
    pub const KHAKI: BColor = BColor::new(240, 230, 140, 255);
    pub const DARKKHAKI: BColor = BColor::new(189, 183, 107, 255);
    pub const LAVENDER: BColor = BColor::new(230, 230, 250, 255);
    pub const THISTLE: BColor = BColor::new(216, 191, 216, 255);
    pub const PLUM: BColor = BColor::new(221, 160, 221, 255);
    pub const VIOLET: BColor = BColor::new(238, 130, 238, 255);
    pub const ORCHID: BColor = BColor::new(218, 112, 214, 255);
    pub const FUCHSIA: BColor = BColor::new(255, 0, 255, 255);
    pub const MAGENTA: BColor = BColor::new(255, 0, 255, 255);
    pub const MEDIUMORCHID: BColor = BColor::new(186, 85, 211, 255);
    pub const MEDIUMPURPLE: BColor = BColor::new(147, 112, 219, 255);
    pub const REBECCAPURPLE: BColor = BColor::new(102, 51, 153, 255);
    pub const BLUEVIOLET: BColor = BColor::new(138, 43, 226, 255);
    pub const DARKVIOLET: BColor = BColor::new(148, 0, 211, 255);
    pub const DARKORCHID: BColor = BColor::new(153, 50, 204, 255);
    pub const DARKMAGENTA: BColor = BColor::new(139, 0, 139, 255);
    pub const PURPLE: BColor = BColor::new(128, 0, 128, 255);
    pub const DARKPURPLE: BColor = BColor::new(112, 31, 126, 255);
    pub const INDIGO: BColor = BColor::new(75, 0, 130, 255);
    pub const SLATEBLUE: BColor = BColor::new(106, 90, 205, 255);
    pub const DARKSLATEBLUE: BColor = BColor::new(72, 61, 139, 255);
    pub const MEDIUMSLATEBLUE: BColor = BColor::new(123, 104, 238, 255);
    pub const GREENYELLOW: BColor = BColor::new(173, 255, 47, 255);
    pub const CHARTREUSE: BColor = BColor::new(127, 255, 0, 255);
    pub const LAWNGREEN: BColor = BColor::new(124, 252, 0, 255);
    pub const LIME: BColor = BColor::new(0, 255, 0, 255);
    pub const LIMEGREEN: BColor = BColor::new(50, 205, 50, 255);
    pub const PALEGREEN: BColor = BColor::new(152, 251, 152, 255);
    pub const LIGHTGREEN: BColor = BColor::new(144, 238, 144, 255);
    pub const MEDIUMSPRINGGREEN: BColor = BColor::new(0, 250, 154, 255);
    pub const SPRINGGREEN: BColor = BColor::new(0, 255, 127, 255);
    pub const MEDIUMSEAGREEN: BColor = BColor::new(60, 179, 113, 255);
    pub const SEAGREEN: BColor = BColor::new(46, 139, 87, 255);
    pub const FORESTGREEN: BColor = BColor::new(34, 139, 34, 255);
    pub const GREEN: BColor = BColor::new(0, 128, 0, 255);
    pub const DARKGREEN: BColor = BColor::new(0, 100, 0, 255);
    pub const YELLOWGREEN: BColor = BColor::new(154, 205, 50, 255);
    pub const OLIVEDRAB: BColor = BColor::new(107, 142, 35, 255);
    pub const OLIVE: BColor = BColor::new(128, 128, 0, 255);
    pub const DARKOLIVEGREEN: BColor = BColor::new(85, 107, 47, 255);
    pub const MEDIUMAQUAMARINE: BColor = BColor::new(102, 205, 170, 255);
    pub const DARKSEAGREEN: BColor = BColor::new(143, 188, 139, 255);
    pub const LIGHTSEAGREEN: BColor = BColor::new(32, 178, 170, 255);
    pub const DARKCYAN: BColor = BColor::new(0, 139, 139, 255);
    pub const TEAL: BColor = BColor::new(0, 128, 128, 255);
    pub const AQUA: BColor = BColor::new(0, 255, 255, 255);
    pub const CYAN: BColor = BColor::new(0, 255, 255, 255);
    pub const LIGHTCYAN: BColor = BColor::new(224, 255, 255, 255);
    pub const PALETURQUOISE: BColor = BColor::new(175, 238, 238, 255);
    pub const AQUAMARINE: BColor = BColor::new(127, 255, 212, 255);
    pub const TURQUOISE: BColor = BColor::new(64, 224, 208, 255);
    pub const MEDIUMTURQUOISE: BColor = BColor::new(72, 209, 204, 255);
    pub const DARKTURQUOISE: BColor = BColor::new(0, 206, 209, 255);
    pub const CADETBLUE: BColor = BColor::new(95, 158, 160, 255);
    pub const STEELBLUE: BColor = BColor::new(70, 130, 180, 255);
    pub const LIGHTSTEELBLUE: BColor = BColor::new(176, 196, 222, 255);
    pub const POWDERBLUE: BColor = BColor::new(176, 224, 230, 255);
    pub const LIGHTBLUE: BColor = BColor::new(173, 216, 230, 255);
    pub const SKYBLUE: BColor = BColor::new(135, 206, 235, 255);
    pub const LIGHTSKYBLUE: BColor = BColor::new(135, 206, 250, 255);
    pub const DEEPSKYBLUE: BColor = BColor::new(0, 191, 255, 255);
    pub const DODGERBLUE: BColor = BColor::new(30, 144, 255, 255);
    pub const CORNFLOWERBLUE: BColor = BColor::new(100, 149, 237, 255);
    pub const ROYALBLUE: BColor = BColor::new(65, 105, 225, 255);
    pub const BLUE: BColor = BColor::new(0, 0, 255, 255);
    pub const MEDIUMBLUE: BColor = BColor::new(0, 0, 205, 255);
    pub const DARKBLUE: BColor = BColor::new(0, 0, 139, 255);
    pub const NAVY: BColor = BColor::new(0, 0, 128, 255);
    pub const MIDNIGHTBLUE: BColor = BColor::new(25, 25, 112, 255);
    pub const CORNSILK: BColor = BColor::new(255, 248, 220, 255);
    pub const BLANCHEDALMOND: BColor = BColor::new(255, 235, 205, 255);
    pub const BISQUE: BColor = BColor::new(255, 228, 196, 255);
    pub const NAVAJOWHITE: BColor = BColor::new(255, 222, 173, 255);
    pub const WHEAT: BColor = BColor::new(245, 222, 179, 255);
    pub const BURLYWOOD: BColor = BColor::new(222, 184, 135, 255);
    pub const TAN: BColor = BColor::new(210, 180, 140, 255);
    pub const ROSYBROWN: BColor = BColor::new(188, 143, 143, 255);
    pub const SANDYBROWN: BColor = BColor::new(244, 164, 96, 255);
    pub const GOLDENROD: BColor = BColor::new(218, 165, 32, 255);
    pub const DARKGOLDENROD: BColor = BColor::new(184, 134, 11, 255);
    pub const PERU: BColor = BColor::new(205, 133, 63, 255);
    pub const CHOCOLATE: BColor = BColor::new(210, 105, 30, 255);
    pub const SADDLEBROWN: BColor = BColor::new(139, 69, 19, 255);
    pub const SIENNA: BColor = BColor::new(160, 82, 45, 255);
    pub const BROWN: BColor = BColor::new(165, 42, 42, 255);
    pub const DARKBROWN: BColor = BColor::new(76, 63, 47, 255);
    pub const MAROON: BColor = BColor::new(128, 0, 0, 255);
    pub const WHITE: BColor = BColor::new(255, 255, 255, 255);
    pub const SNOW: BColor = BColor::new(255, 250, 250, 255);
    pub const HONEYDEW: BColor = BColor::new(240, 255, 240, 255);
    pub const MINTCREAM: BColor = BColor::new(245, 255, 250, 255);
    pub const AZURE: BColor = BColor::new(240, 255, 255, 255);
    pub const ALICEBLUE: BColor = BColor::new(240, 248, 255, 255);
    pub const GHOSTWHITE: BColor = BColor::new(248, 248, 255, 255);
    pub const WHITESMOKE: BColor = BColor::new(245, 245, 245, 255);
    pub const SEASHELL: BColor = BColor::new(255, 245, 238, 255);
    pub const BEIGE: BColor = BColor::new(245, 245, 220, 255);
    pub const OLDLACE: BColor = BColor::new(253, 245, 230, 255);
    pub const FLORALWHITE: BColor = BColor::new(255, 250, 240, 255);
    pub const IVORY: BColor = BColor::new(255, 255, 240, 255);
    pub const ANTIQUEWHITE: BColor = BColor::new(250, 235, 215, 255);
    pub const LINEN: BColor = BColor::new(250, 240, 230, 255);
    pub const LAVENDERBLUSH: BColor = BColor::new(255, 240, 245, 255);
    pub const MISTYROSE: BColor = BColor::new(255, 228, 225, 255);
    pub const GAINSBORO: BColor = BColor::new(220, 220, 220, 255);
    pub const LIGHTGRAY: BColor = BColor::new(211, 211, 211, 255);
    pub const SILVER: BColor = BColor::new(192, 192, 192, 255);
    pub const DARKGRAY: BColor = BColor::new(169, 169, 169, 255);
    pub const GRAY: BColor = BColor::new(128, 128, 128, 255);
    pub const DIMGRAY: BColor = BColor::new(105, 105, 105, 255);
    pub const LIGHTSLATEGRAY: BColor = BColor::new(119, 136, 153, 255);
    pub const SLATEGRAY: BColor = BColor::new(112, 128, 144, 255);
    pub const DARKSLATEGRAY: BColor = BColor::new(47, 79, 79, 255);
    pub const BLACK: BColor = BColor::new(0, 0, 0, 255);
    pub const BLANK: BColor = BColor::new(0, 0, 0, 0);
    pub const RAYWHITE: BColor = BColor::new(245, 245, 245, 255);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_u8(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_f32(&self) -> [f32; 4] {
        self.to_u8().map(|e| e as f32 / 255.0)
    }

    pub fn gray(val: u8) -> Self {
        Self::new(val, val, val, 255)
    }
}
