use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

const BOLD: u8 = 0b0001;
const FAINT: u8 = 0b0010;
const ITALIC: u8 = 0b0100;
const UNDERLINE: u8 = 0b1000;
const INVERSE: u8 = 0b0001_0000;
const HIDDEN: u8 = 0b0010_0000;
const STRIKEOUT: u8 = 0b0100_0000;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Slot {
    #[default]
    Default,
    Ansi(u8),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Style {
    fg: Slot,
    bg: Slot,
    attrs: u8,
}

impl Slot {
    const fn named(color: NamedColor) -> (Self, bool) {
        use NamedColor::{
            Background, Black, Blue, BrightBlack, BrightBlue, BrightCyan, BrightForeground,
            BrightGreen, BrightMagenta, BrightRed, BrightWhite, BrightYellow, Cursor, Cyan,
            DimBlack, DimBlue, DimCyan, DimForeground, DimGreen, DimMagenta, DimRed, DimWhite,
            DimYellow, Foreground, Green, Magenta, Red, White, Yellow,
        };

        match color {
            Black => (Self::Ansi(0), false),
            Red => (Self::Ansi(1), false),
            Green => (Self::Ansi(2), false),
            Yellow => (Self::Ansi(3), false),
            Blue => (Self::Ansi(4), false),
            Magenta => (Self::Ansi(5), false),
            Cyan => (Self::Ansi(6), false),
            White => (Self::Ansi(7), false),
            BrightBlack => (Self::Ansi(8), false),
            BrightRed => (Self::Ansi(9), false),
            BrightGreen => (Self::Ansi(10), false),
            BrightYellow => (Self::Ansi(11), false),
            BrightBlue => (Self::Ansi(12), false),
            BrightMagenta => (Self::Ansi(13), false),
            BrightCyan => (Self::Ansi(14), false),
            BrightWhite => (Self::Ansi(15), false),
            DimBlack => (Self::Ansi(0), true),
            DimRed => (Self::Ansi(1), true),
            DimGreen => (Self::Ansi(2), true),
            DimYellow => (Self::Ansi(3), true),
            DimBlue => (Self::Ansi(4), true),
            DimMagenta => (Self::Ansi(5), true),
            DimCyan => (Self::Ansi(6), true),
            DimWhite => (Self::Ansi(7), true),
            Foreground | BrightForeground | Background | Cursor => (Self::Default, false),
            DimForeground => (Self::Default, true),
        }
    }
}

impl Style {
    pub(super) const fn of(cell: &Cell) -> Self {
        const fn slot(color: Color) -> (Slot, bool) {
            match color {
                Color::Named(named) => Slot::named(named),
                Color::Indexed(index) => (Slot::Indexed(index), false),
                Color::Spec(Rgb {
                    r: red,
                    g: green,
                    b: blue,
                }) => (Slot::Rgb(red, green, blue), false),
            }
        }

        let foreground = slot(cell.fg);
        let background = slot(cell.bg);
        let flags = cell.flags;
        let mut attrs = 0;

        if flags.contains(Flags::BOLD) {
            attrs |= BOLD;
        }

        if foreground.1 || background.1 || flags.contains(Flags::DIM) {
            attrs |= FAINT;
        }

        if flags.contains(Flags::ITALIC) {
            attrs |= ITALIC;
        }

        if flags.intersects(Flags::ALL_UNDERLINES) {
            attrs |= UNDERLINE;
        }

        if flags.contains(Flags::INVERSE) {
            attrs |= INVERSE;
        }

        if flags.contains(Flags::HIDDEN) {
            attrs |= HIDDEN;
        }

        if flags.contains(Flags::STRIKEOUT) {
            attrs |= STRIKEOUT;
        }

        Self {
            fg: foreground.0,
            bg: background.0,
            attrs,
        }
    }

    pub(super) fn push_sgr(self, out: &mut String) {
        if self == Self::default() {
            return;
        }

        let mut params: Vec<u32> = Vec::new();

        if self.attrs & BOLD != 0 {
            params.push(1);
        }

        if self.attrs & FAINT != 0 {
            params.push(2);
        }

        if self.attrs & ITALIC != 0 {
            params.push(3);
        }

        if self.attrs & UNDERLINE != 0 {
            params.push(4);
        }

        if self.attrs & INVERSE != 0 {
            params.push(7);
        }

        if self.attrs & HIDDEN != 0 {
            params.push(8);
        }

        if self.attrs & STRIKEOUT != 0 {
            params.push(9);
        }

        push_slot(&mut params, self.fg, 30, 90, 38);
        push_slot(&mut params, self.bg, 40, 100, 48);

        out.push_str("\x1b[");
        let mut first = true;

        for param in params {
            if !first {
                out.push(';');
            }

            first = false;
            out.push_str(&param.to_string());
        }

        out.push('m');
    }
}

fn push_slot(params: &mut Vec<u32>, slot: Slot, base: u32, bright: u32, extended: u32) {
    match slot {
        Slot::Default => {}
        Slot::Ansi(n) => params.push(if n < 8 {
            base + u32::from(n)
        } else {
            bright + u32::from(n - 8)
        }),
        Slot::Indexed(index) => params.extend([extended, 5, u32::from(index)]),
        Slot::Rgb(red, green, blue) => {
            params.extend([
                extended,
                2,
                u32::from(red),
                u32::from(green),
                u32::from(blue),
            ]);
        }
    }
}
