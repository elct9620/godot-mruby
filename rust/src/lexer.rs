//! The tokens Prism lexes from a Ruby source, which only its C API hands
//! out, read by what colours a line and by what completes a name.

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;

use ruby_prism_sys::{
    pm_lex_callback_t, pm_node_destroy, pm_parse, pm_parser_free, pm_parser_init, pm_parser_t,
    pm_token_t, pm_token_type_t,
};

/// A token Prism lexed: its kind, and the byte offsets of its start and end
/// in the source.
pub struct Token {
    pub kind: pm_token_type_t,
    pub start: usize,
    pub end: usize,
}

/// Every token Prism lexes while it parses the source, in the order they
/// stand in it.
pub fn lex(source: &str) -> Vec<Token> {
    unsafe extern "C" fn collect(
        data: *mut c_void,
        parser: *mut pm_parser_t,
        token: *mut pm_token_t,
    ) {
        // SAFETY: `data` is the vector `lex` hands Prism for this parse, and
        // the parser and token are Prism's, alive for this call.
        let (tokens, source, token) =
            unsafe { (&mut *data.cast::<Vec<Token>>(), (*parser).start, &*token) };
        // SAFETY: a token lies within the source the parser was given.
        let offset = |at: *const u8| unsafe { at.offset_from_unsigned(source) };
        tokens.push(Token {
            kind: token.type_,
            start: offset(token.start),
            end: offset(token.end),
        });
    }

    let mut tokens: Vec<Token> = Vec::new();
    let mut callback = pm_lex_callback_t {
        data: (&raw mut tokens).cast(),
        callback: Some(collect),
    };
    let mut parser = Box::new(MaybeUninit::<pm_parser_t>::uninit());
    // SAFETY: as `ruby_prism::parse` does: the parser is initialised before it
    // is used, the source and callback outlive the parse, and the node and
    // parser are freed once it ends.
    unsafe {
        let parser = parser.as_mut_ptr();
        pm_parser_init(parser, source.as_ptr(), source.len(), ptr::null());
        (*parser).lex_callback = &raw mut callback;
        let node = pm_parse(parser);
        pm_node_destroy(parser, node);
        pm_parser_free(parser);
    }
    tokens
}
