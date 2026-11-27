use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForegroundColor {
    Default,
    Gray,
    Green,
    Cyan,
    Magenta,
    Orange,
    Blue,
    Red,
    Yellow,
    DarkGray,
    Border,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorSpec {
    Default,
    Indexed(u8),
    Rgb((u8, u8, u8)),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemePalette {
    pub default: ColorSpec,
    pub gray: ColorSpec,
    pub green: ColorSpec,
    pub cyan: ColorSpec,
    pub magenta: ColorSpec,
    pub orange: ColorSpec,
    pub blue: ColorSpec,
    pub red: ColorSpec,
    pub yellow: ColorSpec,
    pub dark_gray: ColorSpec,
    pub border: ColorSpec,
    pub selected_foreground: ColorSpec,
    pub selected_background: ColorSpec,
    pub hover_background: ColorSpec,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            default: ColorSpec::Indexed(252),
            gray: ColorSpec::Indexed(245),
            green: ColorSpec::Indexed(46),
            cyan: ColorSpec::Indexed(51),
            magenta: ColorSpec::Indexed(201),
            orange: ColorSpec::Indexed(208),
            blue: ColorSpec::Indexed(75),
            red: ColorSpec::Indexed(196),
            yellow: ColorSpec::Indexed(226),
            dark_gray: ColorSpec::Indexed(240),
            border: ColorSpec::Indexed(240),
            selected_foreground: ColorSpec::Indexed(252),
            selected_background: ColorSpec::Indexed(24),
            hover_background: ColorSpec::Indexed(235),
        }
    }
}

impl ForegroundColor {
    fn ansi_value(self) -> u8 {
        match self {
            Self::Default => 252,
            Self::Gray => 245,
            Self::Green => 46,
            Self::Cyan => 51,
            Self::Magenta => 201,
            Self::Orange => 208,
            Self::Blue => 75,
            Self::Red => 196,
            Self::Yellow => 226,
            Self::DarkGray => 240,
            Self::Border => 240,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyledSpan {
    pub text: String,
    pub foreground: ForegroundColor,
    pub selected_background: bool,
    pub hover_background: bool,
    pub bold: bool,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, foreground: ForegroundColor) -> Self {
        Self {
            text: text.into(),
            foreground,
            selected_background: false,
            hover_background: false,
            bold: false,
        }
    }
}

pub fn truncate_spans(spans: Vec<StyledSpan>, width: usize) -> Vec<StyledSpan> {
    if width == 0 {
        return Vec::new();
    }
    let total_width = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.text.as_str()))
        .sum::<usize>();
    if total_width <= width {
        return spans;
    }

    let content_width = width.saturating_sub(1);
    let mut output = Vec::new();
    let mut used = 0;

    for span in spans {
        let mut text = String::new();
        for character in span.text.chars() {
            let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
            if used + character_width > content_width {
                push_nonempty(&mut output, span_style_with_text(&span, text));
                push_nonempty(
                    &mut output,
                    StyledSpan {
                        text: "…".to_owned(),
                        ..span
                    },
                );
                return output;
            }
            text.push(character);
            used += character_width;
        }
        push_nonempty(&mut output, span_style_with_text(&span, text));
    }

    if let Some(style) = output.last().cloned() {
        push_nonempty(
            &mut output,
            StyledSpan {
                text: "…".to_owned(),
                ..style
            },
        );
    }
    output
}

pub fn pad_spans(mut spans: Vec<StyledSpan>, width: usize) -> Vec<StyledSpan> {
    let visible = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.text.as_str()))
        .sum::<usize>();
    if visible < width {
        spans.push(StyledSpan::new(
            " ".repeat(width - visible),
            ForegroundColor::Default,
        ));
    }
    spans
}

pub fn spans_to_ansi(spans: &[StyledSpan], color: bool) -> String {
    if !color {
        return spans.iter().map(|span| span.text.as_str()).collect();
    }

    let mut output = String::new();
    for span in spans {
        let styled = span.foreground != ForegroundColor::Default
            || span.selected_background
            || span.hover_background
            || span.bold;
        if !styled {
            output.push_str(&span.text);
            continue;
        }

        output.push_str("\u{1b}[");
        let mut first = true;
        if span.bold {
            output.push('1');
            first = false;
        }
        if !first {
            output.push(';');
        }
        output.push_str("38;5;");
        output.push_str(&span.foreground.ansi_value().to_string());
        if span.selected_background {
            output.push_str(";48;5;24");
        } else if span.hover_background {
            output.push_str(";48;5;236");
        }
        output.push('m');
        output.push_str(&span.text);
        output.push_str("\u{1b}[0m");
    }
    output
}

pub fn spans_to_ansi_with_palette(
    spans: &[StyledSpan],
    color: bool,
    palette: &ThemePalette,
) -> String {
    if !color {
        return spans.iter().map(|span| span.text.as_str()).collect();
    }

    let mut output = String::new();
    for span in spans {
        let foreground = match span.foreground {
            ForegroundColor::Default if span.selected_background => palette.selected_foreground,
            ForegroundColor::Default => palette.default,
            ForegroundColor::Gray => palette.gray,
            ForegroundColor::Green => palette.green,
            ForegroundColor::Cyan => palette.cyan,
            ForegroundColor::Magenta => palette.magenta,
            ForegroundColor::Orange => palette.orange,
            ForegroundColor::Blue => palette.blue,
            ForegroundColor::Red => palette.red,
            ForegroundColor::Yellow => palette.yellow,
            ForegroundColor::DarkGray => palette.dark_gray,
            ForegroundColor::Border => palette.border,
        };
        let has_foreground = foreground != ColorSpec::Default;
        let background = if span.selected_background {
            palette.selected_background
        } else if span.hover_background {
            palette.hover_background
        } else {
            ColorSpec::Default
        };
        let has_background = background != ColorSpec::Default;
        if !has_foreground && !has_background && !span.bold {
            output.push_str(&span.text);
            continue;
        }

        let mut codes = Vec::with_capacity(4);
        if span.bold {
            codes.push("1".to_owned());
        }
        if has_foreground {
            codes.push(foreground_sgr(foreground));
        }
        if has_background {
            codes.push(background_sgr(background));
        }
        output.push_str("\u{1b}[");
        output.push_str(&codes.join(";"));
        output.push('m');
        output.push_str(&span.text);
        output.push_str("\u{1b}[0m");
    }
    output
}

fn span_style_with_text(span: &StyledSpan, text: String) -> StyledSpan {
    StyledSpan {
        text,
        ..span.clone()
    }
}

fn push_nonempty(output: &mut Vec<StyledSpan>, span: StyledSpan) {
    if !span.text.is_empty() {
        if let Some(last) = output.last_mut() {
            if last.foreground == span.foreground
                && last.selected_background == span.selected_background
                && last.hover_background == span.hover_background
                && last.bold == span.bold
            {
                last.text.push_str(&span.text);
                return;
            }
        }
        output.push(span);
    }
}

fn foreground_sgr(color: ColorSpec) -> String {
    match color {
        ColorSpec::Default => "39".to_owned(),
        ColorSpec::Indexed(index) => format!("38;5;{index}"),
        ColorSpec::Rgb((red, green, blue)) => format!("38;2;{red};{green};{blue}"),
    }
}

fn background_sgr(color: ColorSpec) -> String {
    match color {
        ColorSpec::Default => "49".to_owned(),
        ColorSpec::Indexed(index) => format!("48;5;{index}"),
        ColorSpec::Rgb((red, green, blue)) => format!("48;2;{red};{green};{blue}"),
    }
}
