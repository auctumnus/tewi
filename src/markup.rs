use argon2::Block;
use askama::Template;
use base64::prelude::*;
use markdown_it::generics::inline;
use uuid::Uuid;

use crate::{
    AppState,
    err::AppResult,
    models::{boards::BoardRepository, posts::PostRepository, threads::ThreadRepository},
    view_structs::{self, components::strudel_code_block},
};

pub struct Render {
    pub content: String,
    pub board_id: Uuid,
}

pub struct MarkupRenderer(AppState);

impl MarkupRenderer {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    async fn transform_inline_node(&self, ctx: &Render, node: InlineNode) -> AppResult<InlineNode> {
        let boards = BoardRepository::new(&self.0);
        let threads = ThreadRepository::new(&self.0);
        let posts = PostRepository::new(&self.0);
        match node {
            InlineNode::PostRef {
                board_name,
                post_number,
            } => {
                if let Some(ref board_name) = board_name {
                    // cross-board reference
                    let board = boards.find_by_slug(board_name).await;

                    match board {
                        Ok(board) => {
                            let post = posts
                                .find_by_board_and_number(
                                    board.id,
                                    post_number.try_into().unwrap_or_default(),
                                )
                                .await?;
                            let thread = threads.find_thread_for_post(post.id).await?;
                            if let Some(op_post) = thread.op_post {
                                let op_post = posts.find_by_id(op_post).await?;
                                Ok(InlineNode::Link {
                                    href: format!(
                                        "/board/{}/thread/{}#post-{}",
                                        board.slug, op_post.post_number, post.post_number
                                    ),
                                    content: vec![InlineNode::Text(format!(
                                        ">>>/{}/{}",
                                        board.slug, post_number
                                    ))],
                                })
                            } else {
                                // thread has no OP post? shouldn't happen, but just in case
                                Ok(InlineNode::Text(format!(
                                    ">>>/{}/{}",
                                    board_name, post_number
                                )))
                            }
                        }
                        Err(e) if e.status_code == 404 => {
                            // board does not exist, return as text
                            Ok(InlineNode::Text(format!(
                                ">>>/{}/{}",
                                board_name, post_number
                            )))
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    // same-board reference
                    let board = boards.find_by_id(ctx.board_id).await?;
                    let post = posts
                        .find_by_board_and_number(
                            ctx.board_id,
                            post_number.try_into().unwrap_or_default(),
                        )
                        .await?;
                    let thread = threads.find_thread_for_post(post.id).await?;
                    if let Some(op_post) = thread.op_post {
                        let op_post = posts.find_by_id(op_post).await?;
                        Ok(InlineNode::Link {
                            href: format!(
                                "/board/{}/thread/{}#post-{}",
                                board.slug, op_post.post_number, post.post_number
                            ),
                            content: vec![InlineNode::Text(format!(">>{}", post_number))],
                        })
                    } else {
                        // thread has no OP post? shouldn't happen, but just in case
                        Ok(InlineNode::Text(format!(">>{}", post_number)))
                    }
                }
            }
            _ => Ok(node),
        }
    }

    pub async fn render(&self, ctx: Render) -> AppResult<String> {
        let mut parser = MarkupParser::new(&ctx.content);
        let mut html_output = String::new();
        for block in parser.parse() {
            match block {
                BlockNode::Paragraph(inline_nodes) => {
                    let mut new_inline_nodes = vec![];
                    for node in inline_nodes {
                        let transformed_node = self.transform_inline_node(&ctx, node).await?;
                        new_inline_nodes.push(transformed_node);
                    }
                    html_output.push_str(&BlockNode::Paragraph(new_inline_nodes).render());
                }
                BlockNode::Quote(inline_nodes) => {
                    let mut new_inline_nodes = vec![];
                    for node in inline_nodes {
                        let transformed_node = self.transform_inline_node(&ctx, node).await?;
                        new_inline_nodes.push(transformed_node);
                    }
                    html_output.push_str(&BlockNode::Quote(new_inline_nodes).render());
                }
                BlockNode::Code(code) => {
                    html_output.push_str(&BlockNode::Code(code).render());
                }
            }
        }
        Ok(html_output)
    }
}

#[derive(Debug)]
pub enum InlineStyle {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Spoiler,
}

#[derive(Debug)]
pub enum InlineNode {
    Text(String),
    Link {
        href: String,
        content: Vec<InlineNode>,
    },
    Styled {
        style: InlineStyle,
        content: Vec<InlineNode>,
    },
    PostRef {
        board_name: Option<String>,
        post_number: u64,
    },
    BoardRef {
        board_name: String,
    },
}

impl InlineNode {
    pub fn render(&self) -> String {
        match self {
            InlineNode::Text(text) => ammonia::clean_text(text),
            InlineNode::Link { href, content } => {
                let inner_html: String = content.iter().map(|node| node.render()).collect();
                format!("<a href=\"{}\">{}</a>", href, inner_html)
            }
            InlineNode::Styled { style, content } => {
                let inner_html: String = content.iter().map(|node| node.render()).collect();
                match style {
                    InlineStyle::Bold => format!("<strong>{}</strong>", inner_html),
                    InlineStyle::Italic => format!("<em>{}</em>", inner_html),
                    InlineStyle::Underline => format!("<u>{}</u>", inner_html),
                    InlineStyle::Strikethrough => format!("<s>{}</s>", inner_html),
                    InlineStyle::Spoiler => {
                        format!("<span class=\"spoiler\">{}</span>", inner_html)
                    }
                }
            }
            InlineNode::PostRef { .. } => "".to_string(), // should be transformed before rendering
            InlineNode::BoardRef { .. } => "".to_string(), // should be transformed before rendering
        }
    }
}

#[derive(Debug)]
enum CodeBlock {
    Shiki(String, Option<String>),
    Strudel(String),
}

#[derive(Debug)]
pub enum BlockNode {
    Paragraph(Vec<InlineNode>),
    Quote(Vec<InlineNode>),
    Code(CodeBlock),
}

impl BlockNode {
    pub fn render(&self) -> String {
        match self {
            BlockNode::Paragraph(inline_nodes) => {
                let inner_html: String = inline_nodes.iter().map(|node| node.render()).collect();
                format!("<p>{}</p>", inner_html)
            }
            BlockNode::Quote(inline_nodes) => {
                let inner_html: String = inline_nodes.iter().map(|node| node.render()).collect();
                format!("<blockquote>{}</blockquote>", inner_html)
            }
            BlockNode::Code(code_block) => match code_block {
                CodeBlock::Shiki(code, language) => {
                    let code_block_id = uuid::Uuid::new_v4();
                    (view_structs::components::shiki_block::ShikiCodeBlockTemplate {
                        block_id: code_block_id,
                        language: BASE64_STANDARD
                            .encode(language.clone().unwrap_or("text".to_string())),
                        encoded_content: BASE64_STANDARD.encode(code),
                        no_script_content: code.to_owned(),
                    })
                    .render()
                    .unwrap_or("<pre>Error rendering code block</pre>".to_string())
                }
                CodeBlock::Strudel(code) => {
                    let code_block_id = uuid::Uuid::new_v4();
                    (view_structs::components::strudel_code_block::StrudelCodeBlockTemplate {
                        block_id: code_block_id,
                        encoded_content: BASE64_STANDARD.encode(code),
                        no_script_content: code.to_owned(),
                    })
                    .render()
                    .unwrap_or("<pre>Error rendering code block</pre>".to_string())
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEndOfInput,
    InvalidPostReference,
    InvalidLink,
}

pub struct MarkupParser {
    pos: usize,
    input: String,
}

const SPECIAL_CHARS: &[char] = &['>', '\n'];

const STRUDEL_HEADER: &str = "strudel```";
const STRUDEL_END: &str = "```";

const CODE_BLOCK_HEADER: &str = "```";
const CODE_BLOCK_END: &str = "```";

impl MarkupParser {
    pub fn new(input: &str) -> Self {
        let input = input.replace("\r\n", "\n").replace("\r", "\n");

        Self { pos: 0, input }
    }

    pub fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.pos)
    }

    pub fn peek2(&self) -> Option<char> {
        self.input.chars().nth(self.pos + 1)
    }

    pub fn peekN(&self, n: usize) -> Option<char> {
        self.input.chars().nth(self.pos + n)
    }

    pub fn rest(&self) -> &str {
        &self.input[self.pos..]
    }

    pub fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    pub fn expect(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    pub fn slug(&mut self) -> String {
        let mut slug = String::new();
        while let Some(ch) = self.peek() {
            if !ch.is_whitespace() && !SPECIAL_CHARS.contains(&ch) {
                slug.push(ch);
                self.pos += 1;
            } else {
                break;
            }
        }
        slug
    }

    pub fn text(&mut self) -> String {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ch == '\\' {
                self.pos += 1;
                if let Some(escaped_char) = self.next() {
                    text.push(escaped_char);
                } else {
                    text.push('\\');
                }
                continue;
            }
            if SPECIAL_CHARS.contains(&ch) {
                break;
            }
            text.push(ch);
            self.pos += 1;
        }
        text
    }

    /// non-consuming
    pub fn strudel_header(&self) -> bool {
        let mut index = 0;
        return STRUDEL_HEADER.chars().all(|c| {
            let is_match = self.peekN(index) == Some(c);
            index = index + 1;
            return is_match;
        });
    }

    /// non-consuming
    pub fn code_header(&self) -> bool {
        let mut index = 0;
        return CODE_BLOCK_HEADER.chars().all(|c| {
            let is_match = self.peekN(index) == Some(c);
            index = index + 1;
            return is_match;
        });
    }

    /// non-consuming
    pub fn code_block_language(&self) -> Option<String> {
        let mut has_lang = false;
        let mut lang = String::new();
        let mut index = self.pos + CODE_BLOCK_HEADER.len();
        while self.peekN(index) != Some('\n') {
            if index >= self.input.len() {
                has_lang = false;
                break;
            }
            if let Some(current_char) = self.peekN(index) {
                has_lang = true;
                lang.push(current_char);
            }
            index += 1;
        }

        return match has_lang {
            true => Some(lang),
            false => None,
        };
    }

    // really the inner part should be Either but Rust doesn't have that
    fn number(&mut self) -> Option<Result<u64, String>> {
        let mut number_str = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                number_str.push(ch);
                self.pos += 1;
            } else {
                break;
            }
        }
        if number_str.is_empty() {
            None
        } else {
            Some(number_str.parse().map_err(|_| number_str))
        }
    }

    fn post_reference(&mut self) -> Option<InlineNode> {
        if self.peek() == Some('>') {
            // This is the third ">" in ">>>/b/..."
            self.pos += 1;
            if self.expect('/') {
                let board_name = self.slug();
                if board_name.is_empty() {
                    return Some(InlineNode::Text(">>>/".to_string()));
                }
                if self.expect('/') {
                    match self.number() {
                        Some(Ok(post_number)) => Some(InlineNode::PostRef {
                            board_name: Some(board_name),
                            post_number,
                        }),
                        Some(Err(evil_number)) => Some(InlineNode::Text(format!(
                            ">>>{}/{}",
                            board_name, evil_number
                        ))),
                        None => Some(InlineNode::BoardRef { board_name }),
                    }
                } else {
                    Some(InlineNode::Text(format!(">>>/{}", board_name)))
                }
            } else {
                Some(InlineNode::Text(">>>".to_string()))
            }
        } else {
            match self.number() {
                Some(Ok(post_number)) => Some(InlineNode::PostRef {
                    board_name: None,
                    post_number,
                }),
                Some(Err(evil_number)) => Some(InlineNode::Text(format!(">>{}", evil_number))),
                None => Some(InlineNode::Text(">>".to_string())),
            }
        }
    }

    /// non-consuming
    fn terminated_block(&self, block_start: &String, block_end: &String) -> Option<(usize, usize)> {
        let mut index = block_start.len();
        let mut no_end = false;
        let mut seek = true;

        while seek {
            if self.pos + index >= self.input.len() {
                no_end = true;
                break;
            }

            let end_found = block_end
                .chars()
                .enumerate()
                .all(|(j, c)| self.peekN(index + j) == Some(c));
            index += 1;
            if end_found {
                seek = false;
            }
        }

        match no_end {
            true => None,
            false => Some((block_start.len(), index - block_start.len())),
        }
    }

    fn inline_node(&mut self) -> Option<InlineNode> {
        match self.peek() {
            // >>123, >>>/b/, >>>/b/123
            Some('>') if self.peek2() == Some('>') => {
                self.pos += 2;
                self.post_reference()
            }
            _ => {
                let text = self.text();
                if text.is_empty() {
                    None
                } else {
                    Some(InlineNode::Text(text))
                }
            }
        }
    }

    fn block_node(&mut self) -> Option<BlockNode> {
        // quote (greentext)
        if self.peek() == Some('>') && self.peek2() != Some('>') {
            self.pos += 1; // consume '>'
            let mut inline_nodes = Vec::new();
            while let Some(inline_node) = self.inline_node() {
                inline_nodes.push(inline_node);
            }
            while self.expect('\n') {}
            Some(BlockNode::Quote(inline_nodes))
        } else if self.strudel_header() {
            match self.terminated_block(&STRUDEL_HEADER.to_string(), &STRUDEL_END.to_string()) {
                Some((offset, end_index)) => {
                    self.pos += offset; // consume 'strudel```'

                    let strudel_body = self
                        .input
                        .chars()
                        .skip(self.pos)
                        .take(end_index - 1)
                        .collect::<String>();

                    self.pos += end_index;
                    self.pos += STRUDEL_END.len(); // consume the length of strudel end marker

                    if self.peek() == Some('\n') {
                        self.pos += 1; // if the character immediately after the block is \n consume it 
                    }

                    Some(BlockNode::Code(CodeBlock::Strudel(strudel_body)))
                }
                None => {
                    // no closing tag so parse the header as text
                    // (this means the rest of the block is parsed as text later)
                    let header_length = STRUDEL_HEADER.len();

                    let text = self
                        .input
                        .chars()
                        .skip(self.pos)
                        .take(header_length)
                        .collect::<String>();
                    self.pos += header_length;

                    if self.peek() == Some('\n') {
                        self.pos += 1; // if the character immediately after the block is \n consume it 
                    }

                    return Some(BlockNode::Paragraph(vec![InlineNode::Text(text)]));
                }
            }
        } else if self.code_header() {
            let lang = self.code_block_language();

            let full_header =
                CODE_BLOCK_HEADER.to_string() + lang.clone().unwrap_or("".to_string()).as_str();

            match self.terminated_block(&full_header, &CODE_BLOCK_END.to_string()) {
                Some((offset, end_index)) => {
                    self.pos += offset; // consume '```${LANG_NAME}'

                    self.pos += 1; // consume the newline character regardless of whether lang was specified

                    let code_body = self
                        .input
                        .chars()
                        .skip(self.pos)
                        .take(end_index - 2)
                        .collect::<String>();

                    self.pos += end_index;
                    self.pos += CODE_BLOCK_END.len(); // consume the length of block end marker

                    if self.peek() == Some('\n') {
                        self.pos += 1; // if the character immediately after the block is \n consume it 
                    }

                    Some(BlockNode::Code(CodeBlock::Shiki(code_body, lang)))
                }
                None => {
                    // no closing tag so parse the header as text
                    // (this means the rest of the block is parsed as text later)
                    let header_length = STRUDEL_HEADER.len();

                    let text = self
                        .input
                        .chars()
                        .skip(self.pos)
                        .take(header_length)
                        .collect::<String>();
                    self.pos += header_length;

                    if self.peek() == Some('\n') {
                        self.pos += 1; // if the character immediately after the block is \n consume it 
                    }

                    return Some(BlockNode::Paragraph(vec![InlineNode::Text(text)]));
                }
            }
        } else {
            let mut inline_nodes = Vec::new();
            while let Some(inline_node) = self.inline_node() {
                inline_nodes.push(inline_node);
            }
            while self.expect('\n') {}

            if inline_nodes.is_empty() {
                None
            } else {
                Some(BlockNode::Paragraph(inline_nodes))
            }
        }
    }

    pub fn parse(&mut self) -> Vec<BlockNode> {
        let mut blocks = Vec::new();
        while let Some(block) = self.block_node() {
            blocks.push(block);
        }
        blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_reference() {
        let mut parser = MarkupParser::new(">>>/b/123");
        let node = parser.inline_node().unwrap();
        match node {
            InlineNode::PostRef {
                board_name,
                post_number,
            } => {
                assert_eq!(board_name.unwrap(), "b");
                assert_eq!(post_number, 123);
            }
            _ => panic!("Expected PostRef, got {:?}", node),
        }
    }

    #[test]
    fn test_post_reference_no_board() {
        let mut parser = MarkupParser::new(">>456");
        let node = parser.inline_node().unwrap();
        match node {
            InlineNode::PostRef {
                board_name,
                post_number,
            } => {
                assert!(board_name.is_none());
                assert_eq!(post_number, 456);
            }
            _ => panic!("Expected PostRef, got {:?}", node),
        }
    }

    #[test]
    fn test_board_reference() {
        let mut parser = MarkupParser::new(">>>/b/");
        let node = parser.inline_node().unwrap();
        match node {
            InlineNode::BoardRef { board_name } => {
                assert_eq!(board_name, "b");
            }
            _ => panic!("Expected BoardRef, got {:?}", node),
        }
    }

    #[test]
    fn test_broken_post_reference_cross_board() {
        let mut parser = MarkupParser::new(">>>/b/abc");
        let node = parser.inline_node().unwrap();
        match node {
            InlineNode::Text(text) => {
                assert_eq!(text, ">>>/b/abc");
            }
            _ => panic!("Expected Text, got {:?}", node),
        }
    }

    #[test]
    fn test_broken_post_reference_no_number() {
        let mut parser = MarkupParser::new(">>");
        let node = parser.inline_node().unwrap();
        match node {
            InlineNode::Text(text) => {
                assert_eq!(text, ">>");
            }
            _ => panic!("Expected Text, got {:?}", node),
        }
    }

    #[test]
    fn test_broken_board_reference_no_name() {
        let mut parser = MarkupParser::new(">>>//123");
        let node = parser.inline_node().unwrap();
        match node {
            InlineNode::Text(text) => {
                assert_eq!(text, ">>>//123");
            }
            _ => panic!("Expected Text, got {:?}", node),
        }
    }

    #[test]
    fn test_broken_board_reference_no_ending_slash() {
        let mut parser = MarkupParser::new(">>>/b");
        let node = parser.inline_node().unwrap();
        match node {
            InlineNode::Text(text) => {
                assert_eq!(text, ">>>/b");
            }
            _ => panic!("Expected Text, got {:?}", node),
        }
    }

    #[test]
    fn test_paragraph_with_text_and_post_ref() {
        let mut parser = MarkupParser::new("Hello >>123 world");
        let blocks = parser.parse();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            BlockNode::Paragraph(inline_nodes) => {
                assert_eq!(inline_nodes.len(), 3);
                match &inline_nodes[0] {
                    InlineNode::Text(text) => assert_eq!(text, "Hello "),
                    _ => panic!("Expected Text, got {:?}", inline_nodes[0]),
                }
                match &inline_nodes[1] {
                    InlineNode::PostRef {
                        board_name,
                        post_number,
                    } => {
                        assert!(board_name.is_none());
                        assert_eq!(*post_number, 123);
                    }
                    _ => panic!("Expected PostRef, got {:?}", inline_nodes[1]),
                }
                match &inline_nodes[2] {
                    InlineNode::Text(text) => assert_eq!(text, " world"),
                    _ => panic!("Expected Text, got {:?}", inline_nodes[2]),
                }
            }
            _ => panic!("Expected Paragraph, got {:?}", blocks[0]),
        }
    }
}
