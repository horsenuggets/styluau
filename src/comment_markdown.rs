use pulldown_cmark::{Event, Options, Parser, Tag};
use std::ops::Range;

#[derive(Default)]
struct Paragraph {
    range: Option<Range<usize>>,
    protected: Vec<Range<usize>>,
    hard_breaks: Vec<Range<usize>>,
    quote_depth: usize,
}

struct Frame {
    inline: bool,
    blocked: bool,
    quote: bool,
}

fn inline_tag(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
    )
}

fn protected_block(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Heading { .. }
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
    )
}

impl Paragraph {
    fn include(&mut self, range: Range<usize>, quote_depth: usize) {
        if let Some(current) = &mut self.range {
            current.start = current.start.min(range.start);
            current.end = current.end.max(range.end);
        } else {
            self.range = Some(range);
            self.quote_depth = quote_depth;
        }
    }

    fn finish(
        &mut self,
        source: &str,
        width: usize,
        replacements: &mut Vec<(Range<usize>, String)>,
    ) {
        let paragraph = std::mem::take(self);
        let Some(range) = paragraph.range else {
            return;
        };
        // Hard breaks end a line on purpose, so each line between them wraps on its own.
        let mut segments = Vec::new();
        let mut start = range.start;
        for hard_break in &paragraph.hard_breaks {
            // A backslash marker stays in its segment so wrapping keeps it on the same line.
            let marker = source[hard_break.clone()].trim_end().len();
            segments.push(start..hard_break.start + marker);
            start = skip_quote_prefix(source, hard_break.end, range.end, paragraph.quote_depth);
            while start < range.end && matches!(source.as_bytes()[start], b' ' | b'\t') {
                start += 1;
            }
        }
        segments.push(start..range.end);
        for segment in segments {
            if segment.is_empty() {
                continue;
            }
            let protected = paragraph
                .protected
                .iter()
                .filter(|span| span.start >= segment.start && span.end <= segment.end)
                .cloned()
                .collect();
            if let Some(reflowed) =
                format_paragraph(source, &segment, protected, paragraph.quote_depth, width)
            {
                if reflowed != source[segment.clone()] {
                    replacements.push((segment, reflowed));
                }
            }
        }
    }
}

/// Drops whitespace after a line-ending backslash so it still forms a Markdown hard break.
fn normalize_backslash_breaks(source: &str) -> String {
    source
        .split('\n')
        .map(|line| {
            let trimmed = line.trim_end_matches([' ', '\t']);
            let backslashes = trimmed
                .bytes()
                .rev()
                .take_while(|byte| *byte == b'\\')
                .count();
            if backslashes % 2 == 1 {
                trimmed
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Reflows paragraph prose without rendering or rewriting Markdown block syntax.
pub(crate) fn reflow_markdown(source: &str, width: usize) -> String {
    let normalized = normalize_backslash_breaks(source);
    let source = normalized.as_str();
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut frames = Vec::<Frame>::new();
    let mut paragraph = Paragraph::default();
    let mut replacements = Vec::new();
    let mut quote_depth = 0;

    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        let blocked = frames.last().is_some_and(|frame| frame.blocked);
        match event {
            Event::Start(tag) => {
                let inline = inline_tag(&tag);
                let quote = matches!(tag, Tag::BlockQuote(_));
                if !inline {
                    paragraph.finish(source, width, &mut replacements);
                } else if !blocked {
                    paragraph.include(range.clone(), quote_depth);
                    if matches!(tag, Tag::Link { .. } | Tag::Image { .. }) {
                        paragraph.protected.push(range);
                    }
                }
                frames.push(Frame {
                    inline,
                    blocked: blocked || protected_block(&tag),
                    quote,
                });
                if quote {
                    quote_depth += 1;
                }
            }
            Event::End(_) => {
                if let Some(frame) = frames.pop() {
                    if !frame.inline {
                        paragraph.finish(source, width, &mut replacements);
                    } else if !frame.blocked {
                        paragraph.include(range, quote_depth);
                    }
                    if frame.quote {
                        quote_depth -= 1;
                    }
                }
            }
            Event::TaskListMarker(_) => {
                // Keep the marker outside the paragraph so continuation lines align with prose.
            }
            Event::Text(_) | Event::SoftBreak if !blocked => {
                paragraph.include(range, quote_depth);
            }
            Event::Code(_) | Event::InlineHtml(_) if !blocked => {
                paragraph.include(range.clone(), quote_depth);
                paragraph.protected.push(range);
            }
            Event::HardBreak if !blocked => {
                paragraph.include(range.clone(), quote_depth);
                paragraph.hard_breaks.push(range);
            }
            _ => {
                if !blocked {
                    paragraph.finish(source, width, &mut replacements);
                }
            }
        }
    }
    paragraph.finish(source, width, &mut replacements);

    let mut output = String::with_capacity(source.len());
    let mut end = 0;
    for (range, replacement) in replacements {
        output.push_str(&source[end..range.start]);
        output.push_str(&replacement);
        end = range.end;
    }
    output.push_str(&source[end..]);
    output
}

fn continuation_prefix(prefix: &str) -> String {
    prefix
        .chars()
        .map(|character| match character {
            '>' | ' ' | '\t' => character,
            _ => ' ',
        })
        .collect()
}

fn skip_quote_prefix(source: &str, mut index: usize, end: usize, depth: usize) -> usize {
    for _ in 0..depth {
        let start = index;
        while index < end && matches!(source.as_bytes()[index], b' ' | b'\t') {
            index += 1;
        }
        if index < end && source.as_bytes()[index] == b'>' {
            index += 1;
        } else {
            index = start;
            break;
        }
    }
    index
}

fn syntax_at_line_start(word: &str) -> bool {
    matches!(word, "+" | "#" | "##" | "###" | "####" | "#####" | "######")
        || word.starts_with('<')
        || word.starts_with('>')
        || word.starts_with('|')
        || (word.starts_with('[') && (!word.contains(']') || word.contains("]:")))
        || (word.contains('-') && word.bytes().all(|byte| matches!(byte, b'-' | b':' | b'|')))
        || word.starts_with("```")
        || word.starts_with("~~~")
        || b"-*_="
            .iter()
            .any(|marker| !word.is_empty() && word.bytes().all(|character| character == *marker))
        || word.strip_suffix(['.', ')']).is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn format_paragraph(
    source: &str,
    range: &Range<usize>,
    mut protected: Vec<Range<usize>>,
    quote_depth: usize,
    width: usize,
) -> Option<String> {
    let line_start = source[..range.start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let prefix = &source[line_start..range.start];
    let continuation = continuation_prefix(prefix);
    protected.sort_by_key(|range| (range.start, std::cmp::Reverse(range.end)));
    let mut spans = Vec::<Range<usize>>::new();
    for span in protected {
        if let Some(last) = spans.last_mut() {
            if span.start <= last.end {
                last.end = last.end.max(span.end);
                continue;
            }
        }
        spans.push(span);
    }

    let mut words = Vec::new();
    let mut word = String::new();
    let mut position = range.start;
    let mut span_index = 0;
    while position < range.end {
        if let Some(span) = spans.get(span_index) {
            if position == span.start {
                let text = &source[span.clone()];
                // A multiline code/link atom has intentional layout that must not be rewritten.
                if text.contains('\n') {
                    return None;
                }
                word.push_str(text);
                position = span.end;
                span_index += 1;
                continue;
            }
        }
        let character = source[position..].chars().next().unwrap();
        position += character.len_utf8();
        if character.is_ascii_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            if character == '\n' {
                position = skip_quote_prefix(source, position, range.end, quote_depth);
            }
        } else {
            word.push(character);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    // A separated hard break marker must never wrap onto a line by itself.
    if words.len() > 1 && words.last().is_some_and(|word| word == "\\") {
        words.pop();
        if let Some(last) = words.last_mut() {
            last.push_str(" \\");
        }
    }

    let mut output = String::new();
    let mut used_width = prefix.chars().count();
    let mut line_has_word = false;
    for word in words {
        let word_width = word.chars().count();
        let separator_width = usize::from(line_has_word);
        // Breaking after an odd backslash run would introduce a Markdown hard break.
        let escaped_line_end = (output
            .bytes()
            .rev()
            .take_while(|byte| *byte == b'\\')
            .count()
            & 1)
            != 0;
        if line_has_word
            && used_width + separator_width + word_width > width
            && !syntax_at_line_start(&word)
            && !escaped_line_end
        {
            output.push('\n');
            output.push_str(&continuation);
            used_width = continuation.chars().count();
            line_has_word = false;
        }
        if line_has_word {
            output.push(' ');
            used_width += 1;
        }
        output.push_str(&word);
        used_width += word_width;
        line_has_word = true;
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::reflow_markdown;
    use pulldown_cmark::{Event, Options, Parser};

    fn structure(source: &str) -> Vec<String> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
        let mut result = Vec::new();
        let mut text = String::new();
        for event in Parser::new_ext(source, options) {
            match event {
                Event::Text(value) => {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(&value.split_whitespace().collect::<Vec<_>>().join(" "));
                }
                Event::SoftBreak => (),
                event => {
                    if !text.is_empty() {
                        result.push(format!("Text {}", std::mem::take(&mut text)));
                    }
                    result.push(format!("{:?}", event));
                }
            }
        }
        if !text.is_empty() {
            result.push(format!("Text {}", text));
        }
        result
    }

    fn check(input: &str, width: usize, expected: &str) {
        let result = reflow_markdown(input, width);
        assert_eq!(result, expected);
        assert_eq!(reflow_markdown(&result, width), result);
    }

    #[test]
    fn joins_soft_breaks_and_preserves_paragraphs() {
        check(
            "These short lines\nbelong to one paragraph.\n\nThis is separate.\n",
            31,
            "These short lines belong to one\nparagraph.\n\nThis is separate.\n",
        );
        check(
            "First paragraph.\n\n\nSecond paragraph.",
            90,
            "First paragraph.\n\n\nSecond paragraph.",
        );
    }

    #[test]
    fn keeps_list_items_separate_and_hangs_continuations() {
        check(
            "- First item has several words that should wrap.\n- Second item stays separate.\n",
            29,
            "- First item has several\n  words that should wrap.\n- Second item stays separate.\n",
        );
        check(
            "3. First numbered item has several words.\n4. Second numbered item has several words.\n",
            28,
            "3. First numbered item has\n   several words.\n4. Second numbered item has\n   several words.\n",
        );
        check(
            "1) First item has several words that wrap.\n2) Another item.\n",
            25,
            "1) First item has several\n   words that wrap.\n2) Another item.\n",
        );
    }

    #[test]
    fn keeps_nested_lists_and_original_markers() {
        check(
            "* Parent item with several words.\n  + Nested item with several more words.\n* Other parent.\n",
            25,
            "* Parent item with\n  several words.\n  + Nested item with\n    several more words.\n* Other parent.\n",
        );
        check(
            "- [x] Finished task with several words.\n- [ ] Remaining task.\n",
            26,
            "- [x] Finished task with\n      several words.\n- [ ] Remaining task.\n",
        );
    }

    #[test]
    fn reflows_blockquotes_without_losing_prefixes() {
        check(
            "> This quote has several\n> lines in the same paragraph.\n>\n> Another paragraph.\n",
            27,
            "> This quote has several\n> lines in the same\n> paragraph.\n>\n> Another paragraph.\n",
        );
        check(
            "> - Quoted list item with many words to wrap.\n> - Another item.\n",
            27,
            "> - Quoted list item with\n>   many words to wrap.\n> - Another item.\n",
        );
    }

    #[test]
    fn preserves_non_prose_blocks_and_definitions() {
        let source = "# A long heading remains unchanged\n\nHeading with underline\n======================\n\n---\n\n```luau\nconst    value= 1\n```\n\n~~~\n  another   example\n~~~\n\n    indented    code\n    another      line\n\n| First | Second |\n| --- | --- |\n| Long cell contents | Other content |\n\n[link]: https://example.invalid/a/very/long/path \"Reference title\"\n";
        check(source, 12, source);
    }

    #[test]
    fn keeps_inline_code_links_urls_and_markup() {
        check(
            "Use `a code span with spaces` and [a link label](https://example.invalid/path) here.",
            25,
            "Use\n`a code span with spaces`\nand\n[a link label](https://example.invalid/path)\nhere.",
        );
        check(
            "A sentence with **bold words** and *emphasized words*.",
            24,
            "A sentence with **bold\nwords** and *emphasized\nwords*.",
        );
        check(
            "Visit https://example.invalid/a/long/path now.",
            18,
            "Visit\nhttps://example.invalid/a/long/path\nnow.",
        );
        check(
            "Use `a\nmultiline code span` here.",
            12,
            "Use `a\nmultiline code span` here.",
        );
    }

    #[test]
    fn preserves_explicit_hard_breaks() {
        for source in [
            "This line has two spaces.  \nThis line stays separate.\n",
            "This line has a backslash.\\\nThis line stays separate.\n",
            "This line has a spaced backslash. \\\nThis line stays separate.\n",
        ] {
            check(source, 90, source);
        }
    }

    #[test]
    fn joins_lines_after_a_hard_break_but_not_across_it() {
        check("a \\\nb\nc\n", 90, "a \\\nb c\n");
        check("a\nb \\\nc\n", 90, "a b \\\nc\n");
        check("first  \nsecond\nthird\n", 90, "first  \nsecond third\n");
    }

    #[test]
    fn drops_whitespace_after_a_backslash_break() {
        check("a \\ \nb\n", 90, "a \\\nb\n");
        check("a \\\t\nb\n", 90, "a \\\nb\n");
    }

    #[test]
    fn wraps_each_hard_break_segment_independently() {
        check(
            "one two three \\\nfour five\n",
            9,
            "one two\nthree \\\nfour five\n",
        );
        check("first  \nsecond third\n", 8, "first  \nsecond\nthird\n");
        check(
            "- first \\\n  second third\n",
            10,
            "- first \\\n  second\n  third\n",
        );
        check(
            "> first \\\n> second third\n",
            10,
            "> first \\\n> second\n> third\n",
        );
    }

    #[test]
    fn keeps_a_backslash_marker_with_its_last_word() {
        check("one two \\\nthree\n", 7, "one\ntwo \\\nthree\n");
        check("one two\\\nthree\n", 7, "one\ntwo\\\nthree\n");
    }

    #[test]
    fn treats_escaped_backslashes_as_text() {
        check("a \\\\\nb\n", 90, "a \\\\ b\n");
        check("a \\\\ \nb\n", 90, "a \\\\ b\n");
    }

    #[test]
    fn wraps_unicode_at_character_boundaries_and_handles_narrow_widths() {
        check(
            "Café déjà vu and naïve words.",
            12,
            "Café déjà vu\nand naïve\nwords.",
        );
        check("one two three", 1, "one\ntwo\nthree");
        check("", 1, "");
    }

    #[test]
    fn preserves_markdown_structure_and_is_idempotent_across_widths() {
        for source in [
            "Ordinary prose with several words and a marker - within this paragraph.",
            "A path ends in C:\\ and has more words.",
            r"A path ends in C:\\ and has more words.",
            r"A path ends in C:\\\ and has more words.",
            "Words before == an underline mentioned in prose.",
            "Words before -- an underline mentioned in prose.",
            "Words before >quoted text mentioned in prose.",
            "Words before _ _ _ markers mentioned in prose.",
            "Words before [label]: destination mentioned in prose.",
            "Words before [long label]: destination mentioned in prose.",
            "Words before alpha | beta | --- | --- and more prose.",
            "A sentence with **bold words** and *emphasized words* and `inline code`.",
            "- First list item with enough words to wrap.\n- Another item with words.\n",
            "1. First ordered item with enough words.\n2. Another numbered item.\n",
            "> A quoted paragraph with enough words\n> and a soft continuation.\n",
            "- Parent item with words.\n  - Nested item with words.\n\n    Another nested paragraph with words.\n",
            "Use [a descriptive link label](https://example.invalid/path \"A title\") here.",
            "First line with several words \\\nsecond line with more words.\n",
            "First line with several words\\\nsecond line with more words.\n",
            "First line with several words  \nsecond line with more words.\n",
            "- Item with several words \\\n  continued item with words.\n",
            "> Quoted with several words \\\n> more quoted words here.\n",
        ] {
            for width in 1..60 {
                let result = reflow_markdown(source, width);
                assert_eq!(reflow_markdown(&result, width), result, "Width {} and source {:?}", width, source);
                assert_eq!(structure(source), structure(&result), "Width {} and source {:?}", width, source);
            }
        }
    }

    #[test]
    fn does_not_turn_prose_into_a_new_markdown_block() {
        check(
            "Words before - a marker in prose.",
            13,
            "Words before -\na marker in\nprose.",
        );
        check(
            "Words before 1. a marker in prose.",
            13,
            "Words before 1.\na marker in\nprose.",
        );
    }
}
