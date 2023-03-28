// Hi! Thanks for creating the `tree-sitter-c2rust` crate! I am currently trying to get my crate, which depends on tree-sitter, to compile to wasm. Using your crate, I got it working. However, besides the base `tree-sitter` crate, I also depend on `tree-sitter-highlight` (and `tree-sitter-loader` for non-wasm targets). Since you haven't published the c2rust versions of these to crates.io as you did with the main crate, I currently have to define all three dependencies with git urls. This makes publishing my crate to crates.io impossible. Could you potentially just publish all crates in the tree-sitter-c2rust repo to crates.io? Thanks in advance
//
//
// use std::{sync::atomic::AtomicUsize, iter, ops};
//
// use anyhow::Result;
// use tree_sitter::{Parser, QueryCursor, Query, Language, Tree, QueryCaptures, Range, Point};
//
// pub struct Highlighter {
//     parser: Parser,
//     cursors: Vec<QueryCursor>,
// }
//
// /// Indicates which highlight should be applied to a region of source code.
// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
// pub struct Highlight(pub usize);
//
// /// Contains the data needed to highlight code written in a particular language.
// ///
// /// This struct is immutable and can be shared between threads.
// pub struct HighlightConfiguration {
//     pub language: Language,
//     pub query: Query,
//     combined_injections_query: Option<Query>,
//     locals_pattern_index: usize,
//     highlights_pattern_index: usize,
//     highlight_indices: Vec<Option<Highlight>>,
//     non_local_variable_patterns: Vec<bool>,
//     injection_content_capture_index: Option<u32>,
//     injection_language_capture_index: Option<u32>,
//     local_scope_capture_index: Option<u32>,
//     local_def_capture_index: Option<u32>,
//     local_def_value_capture_index: Option<u32>,
//     local_ref_capture_index: Option<u32>,
// }
//
// /// Represents a single step in rendering a syntax-highlighted document.
// #[derive(Copy, Clone, Debug)]
// pub enum HighlightEvent {
//     Source { start: usize, end: usize },
//     HighlightStart(Highlight),
//     HighlightEnd,
// }
//
// struct HighlightIterLayer<'a> {
//     _tree: Tree,
//     cursor: QueryCursor,
//     captures: iter::Peekable<QueryCaptures<'a, 'a, &'a [u8]>>,
//     config: &'a HighlightConfiguration,
//     highlight_end_stack: Vec<usize>,
//     scope_stack: Vec<LocalScope<'a>>,
//     ranges: Vec<Range>,
//     depth: usize,
// }
//
// #[derive(Debug)]
// struct LocalScope<'a> {
//     inherits: bool,
//     range: ops::Range<usize>,
//     local_defs: Vec<LocalDef<'a>>,
// }
//
// #[derive(Debug)]
// struct LocalDef<'a> {
//     name: &'a str,
//     value_range: ops::Range<usize>,
//     highlight: Option<Highlight>,
// }
//
// impl Highlighter {
//     pub fn new() -> Self {
//         Self {
//             parser: Parser::new(),
//             cursors: vec![],
//         }
//     }
//
//     /// Iterate over the highlighted regions for a given slice of source code.
//     pub fn highlight<'a>(
//         &mut self,
//         config: &'a HighlightConfiguration,
//         source: &'a [u8],
//         cancellation_flag: Option<&'a AtomicUsize>,
//         mut injection_callback: impl FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
//     ) -> Result<impl Iterator<Item = Result<HighlightEvent>> + 'a> {
//         let layers = HighlightIterLayer::new(
//             source,
//             self,
//             cancellation_flag,
//             &mut injection_callback,
//             config,
//             0,
//             vec![Range {
//                 start_byte: 0,
//                 end_byte: usize::MAX,
//                 start_point: Point::new(0, 0),
//                 end_point: Point::new(usize::MAX, usize::MAX),
//             }],
//         )?;
//         assert_ne!(layers.len(), 0);
//         let mut result = HighlightIter {
//             source,
//             byte_offset: 0,
//             injection_callback,
//             cancellation_flag,
//             highlighter: self,
//             iter_count: 0,
//             layers,
//             next_event: None,
//             last_highlight_range: None,
//         };
//         result.sort_layers();
//         Ok(result)
//     }
// }
//
// impl<'a> HighlightIterLayer<'a> {
//     /// Create a new 'layer' of highlighting for this document.
//     ///
//     /// In the even that the new layer contains "combined injections" (injections where multiple
//     /// disjoint ranges are parsed as one syntax tree), these will be eagerly processed and
//     /// added to the returned vector.
//     fn new<F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a>(
//         source: &'a [u8],
//         highlighter: &mut Highlighter,
//         cancellation_flag: Option<&'a AtomicUsize>,
//         injection_callback: &mut F,
//         mut config: &'a HighlightConfiguration,
//         mut depth: usize,
//         mut ranges: Vec<Range>,
//     ) -> Result<Vec<Self>> {
//         let mut result = Vec::with_capacity(1);
//         let mut queue = Vec::new();
//         loop {
//             if highlighter.parser.set_included_ranges(&ranges).is_ok() {
//                 highlighter
//                     .parser
//                     .set_language(config.language)
//                     .map_err(|_| Error::InvalidLanguage)?;
//
//                 unsafe { highlighter.parser.set_cancellation_flag(cancellation_flag) };
//                 let tree = highlighter
//                     .parser
//                     .parse(source, None)
//                     .ok_or(Error::Cancelled)?;
//                 unsafe { highlighter.parser.set_cancellation_flag(None) };
//                 let mut cursor = highlighter.cursors.pop().unwrap_or(QueryCursor::new());
//
//                 // Process combined injections.
//                 if let Some(combined_injections_query) = &config.combined_injections_query {
//                     let mut injections_by_pattern_index =
//                         vec![(None, Vec::new(), false); combined_injections_query.pattern_count()];
//                     let matches =
//                         cursor.matches(combined_injections_query, tree.root_node(), source);
//                     for mat in matches {
//                         let entry = &mut injections_by_pattern_index[mat.pattern_index];
//                         let (language_name, content_node, include_children) =
//                             injection_for_match(config, combined_injections_query, &mat, source);
//                         if language_name.is_some() {
//                             entry.0 = language_name;
//                         }
//                         if let Some(content_node) = content_node {
//                             entry.1.push(content_node);
//                         }
//                         entry.2 = include_children;
//                     }
//                     for (lang_name, content_nodes, includes_children) in injections_by_pattern_index
//                     {
//                         if let (Some(lang_name), false) = (lang_name, content_nodes.is_empty()) {
//                             if let Some(next_config) = (injection_callback)(lang_name) {
//                                 let ranges = Self::intersect_ranges(
//                                     &ranges,
//                                     &content_nodes,
//                                     includes_children,
//                                 );
//                                 if !ranges.is_empty() {
//                                     queue.push((next_config, depth + 1, ranges));
//                                 }
//                             }
//                         }
//                     }
//                 }
//
//                 // The `captures` iterator borrows the `Tree` and the `QueryCursor`, which
//                 // prevents them from being moved. But both of these values are really just
//                 // pointers, so it's actually ok to move them.
//                 let tree_ref = unsafe { mem::transmute::<_, &'static Tree>(&tree) };
//                 let cursor_ref =
//                     unsafe { mem::transmute::<_, &'static mut QueryCursor>(&mut cursor) };
//                 let captures = cursor_ref
//                     .captures(&config.query, tree_ref.root_node(), source)
//                     .peekable();
//
//                 result.push(HighlightIterLayer {
//                     highlight_end_stack: Vec::new(),
//                     scope_stack: vec![LocalScope {
//                         inherits: false,
//                         range: 0..usize::MAX,
//                         local_defs: Vec::new(),
//                     }],
//                     cursor,
//                     depth,
//                     _tree: tree,
//                     captures,
//                     config,
//                     ranges,
//                 });
//             }
//
//             if queue.is_empty() {
//                 break;
//             } else {
//                 let (next_config, next_depth, next_ranges) = queue.remove(0);
//                 config = next_config;
//                 depth = next_depth;
//                 ranges = next_ranges;
//             }
//         }
//
//         Ok(result)
//     }
// }
