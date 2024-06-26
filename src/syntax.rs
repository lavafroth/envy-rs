use color_eyre::eyre::Result;
use std::{io, sync::OnceLock};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder},
    util::as_24_bit_terminal_escaped,
};

static THEME: OnceLock<Theme> = OnceLock::new();
static SYNTAX: OnceLock<SyntaxSet> = OnceLock::new();

pub fn highlight(s: &str) -> Result<String> {
    let theme = THEME.get_or_init(|| {
        let theme_data: &'static [u8] = include_bytes!("MonokaiDarkSoda.tmTheme");
        let mut theme_reader: io::Cursor<&[u8]> = io::Cursor::new(theme_data);
        ThemeSet::load_from_reader(&mut theme_reader).unwrap()
    });
    let syntax = SYNTAX.get_or_init(|| {
        let syntax: SyntaxDefinition = SyntaxDefinition::load_from_str(
            include_str!("PowerShellSyntax.sublime-syntax"),
            false,
            None,
        )
        .unwrap();
        let mut syntax_set_builder = SyntaxSetBuilder::new();
        syntax_set_builder.add(syntax);
        syntax_set_builder.build()
    });

    let mut h = HighlightLines::new(syntax.find_syntax_by_name("PowerShell").unwrap(), theme);
    let ranges: Vec<(Style, &str)> = h.highlight_line(s, syntax)?;
    Ok(as_24_bit_terminal_escaped(&ranges[..], false))
}
