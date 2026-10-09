use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::{
    BinOp, Expr, ExprBinary, ExprCall, ExprClosure, Ident, Lit, LitStr, Pat, RangeLimits, Result,
    ReturnType, Token, UnOp, braced, parenthesized,
    parse::{ParseStream, Parser, discouraged::Speculative},
    parse2,
    spanned::Spanned,
    token,
};

enum GroupKind {
    All,
    Any,
    Not,
}

enum CardKind {
    Exactly,
    AtMost,
    AtLeast,
}

enum Item {
    Leaf(Expr),
    Member(Expr, Expr),
    Label(LitStr, Box<Item>),
    Group(GroupKind, Vec<Item>),
    Implies(Vec<Item>, Vec<Item>),
    Card(CardKind, Expr, Vec<Item>),
    Distinct(Expr),
    For(Pat, Expr, Vec<Item>),
    If(Expr, Vec<Item>, Option<Vec<Item>>),
}

const IMPLIES_HELP: &str = "`implies` needs a block on both sides: `{ p } implies { q }`";

// ==== Parser ====================

fn peek_ident(input: ParseStream, name: &str) -> bool {
    input.peek(Ident) && input.fork().parse::<Ident>().is_ok_and(|i| i == name)
}

fn braced_items(input: ParseStream) -> Result<Vec<Item>> {
    let content;
    braced!(content in input);
    parse_items(&content)
}

fn parse_items(input: ParseStream) -> Result<Vec<Item>> {
    let mut items = Vec::new();
    while !input.is_empty() {
        let (item, block_like) = parse_item(input)?;
        items.push(item);
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else if !input.is_empty() && !block_like {
            return Err(input.error("expected `,` between constraints"));
        }
    }
    Ok(items)
}

fn parse_item(input: ParseStream) -> Result<(Item, bool)> {
    if input.peek(Token![for]) {
        input.parse::<Token![for]>()?;
        let pat = Pat::parse_multi_with_leading_vert(input)?;
        input.parse::<Token![in]>()?;
        let iter = Expr::parse_without_eager_brace(input)?;
        let body = braced_items(input)?;
        return Ok((Item::For(pat, iter, body), true));
    }
    if input.peek(Token![if]) {
        return Ok((parse_if(input)?, true));
    }
    // label:
    if input.peek(LitStr) && input.peek2(Token![:]) {
        let label: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;
        let (item, block_like) = parse_or(input)?;
        return Ok((Item::Label(label, Box::new(item)), block_like));
    }
    parse_or(input)
}

fn parse_if(input: ParseStream) -> Result<Item> {
    input.parse::<Token![if]>()?;
    let cond = Expr::parse_without_eager_brace(input)?;
    let then = braced_items(input)?;
    let els = if input.peek(Token![else]) {
        input.parse::<Token![else]>()?;
        if input.peek(Token![if]) {
            Some(vec![parse_if(input)?])
        } else {
            Some(braced_items(input)?)
        }
    } else {
        None
    };
    Ok(Item::If(cond, then, els))
}

// `expr`, `x in iter()`
fn parse_leaf(input: ParseStream) -> Result<Item> {
    let expr: Expr = input.parse()?;
    if input.peek(Token![in]) {
        input.parse::<Token![in]>()?;
        let rhs: Expr = input.parse()?;
        return Ok(Item::Member(expr, rhs));
    }
    if peek_ident(input, "implies") {
        return Err(input.error(IMPLIES_HELP));
    }
    if !input.is_empty() {
        return Err(input.error("expected `,`, `&&` or `||` after a constraint"));
    }
    Ok(Item::Leaf(expr))
}

fn at_boundary(input: ParseStream) -> bool {
    input.is_empty() || input.peek(Token![,]) || input.peek(Token![&&]) || input.peek(Token![||])
}

fn parse_atom(input: ParseStream) -> Result<Item> {
    let mut ts = TokenStream::new();
    while !at_boundary(input) {
        ts.extend([input.parse::<TokenTree>()?]);
    }
    if ts.is_empty() {
        return Err(input.error("expected a constraint"));
    }
    Parser::parse2(parse_leaf, ts)
}

fn chain(
    input: ParseStream,
    kind: fn() -> GroupKind,
    peek: fn(ParseStream) -> bool,
    eat: fn(ParseStream) -> Result<()>,
    next: fn(ParseStream) -> Result<(Item, bool)>,
) -> Result<(Item, bool)> {
    let (first, mut block_like) = next(input)?;
    if !peek(input) {
        return Ok((first, block_like));
    }
    let mut terms = vec![first];
    while peek(input) {
        eat(input)?;
        let (t, b) = next(input)?;
        terms.push(t);
        block_like = b;
    }
    Ok((Item::Group(kind(), terms), block_like))
}

fn parse_or(input: ParseStream) -> Result<(Item, bool)> {
    chain(
        input,
        || GroupKind::Any,
        |i| i.peek(Token![||]),
        |i| i.parse::<Token![||]>().map(drop),
        parse_and,
    )
}

fn parse_and(input: ParseStream) -> Result<(Item, bool)> {
    chain(
        input,
        || GroupKind::All,
        |i| i.peek(Token![&&]),
        |i| i.parse::<Token![&&]>().map(drop),
        parse_unary,
    )
}

fn parse_unary(input: ParseStream) -> Result<(Item, bool)> {
    if input.peek(Token![!]) {
        input.parse::<Token![!]>()?;
        let (inner, block_like) = parse_unary(input)?;
        return Ok((Item::Group(GroupKind::Not, vec![inner]), block_like));
    }
    parse_primary(input)
}

fn parse_primary(input: ParseStream) -> Result<(Item, bool)> {
    // `(` or `)`
    if input.peek(token::Paren) {
        let fork = input.fork();
        let content;
        parenthesized!(content in fork);
        let inner = parse_or(&content).ok().filter(|_| content.is_empty());
        if let Some((item, _)) = inner
            && at_boundary(&fork)
        {
            input.advance_to(&fork);
            return Ok((item, false));
        }
    }
    // ` { p } implies { q }`
    if input.peek(token::Brace) {
        let fork = input.fork();
        let _skipped;
        braced!(_skipped in fork);
        if peek_ident(&fork, "implies") {
            let lhs = braced_items(input)?;
            let kw: Ident = input.parse()?;
            if !input.peek(token::Brace) {
                return Err(syn::Error::new(kw.span(), IMPLIES_HELP));
            }
            let rhs = braced_items(input)?;
            return Ok((Item::Implies(lhs, rhs), true));
        }
    }
    if input.peek(Ident) {
        let fork = input.fork();
        let name: Ident = fork.parse()?;
        let name = name.to_string();
        match name.as_str() {
            // `any { z == 5, z == 8 }`
            "all" | "any" | "not" if fork.peek(token::Brace) => {
                let kind = match name.as_str() {
                    "all" => GroupKind::All,
                    "any" => GroupKind::Any,
                    _ => GroupKind::Not,
                };
                input.parse::<Ident>()?;
                let body = braced_items(input)?;
                return Ok((Item::Group(kind, body), true));
            }
            // `exactly(k) { for x in &xs { item[x] } }`
            "exactly" | "at_most" | "at_least" if fork.peek(token::Paren) => {
                let inner;
                parenthesized!(inner in fork);
                if inner.parse::<Expr>().is_ok() && fork.peek(token::Brace) {
                    let kind = match name.as_str() {
                        "exactly" => CardKind::Exactly,
                        "at_most" => CardKind::AtMost,
                        _ => CardKind::AtLeast,
                    };
                    input.parse::<Ident>()?;
                    let k_content;
                    parenthesized!(k_content in input);
                    let k: Expr = k_content.parse()?;
                    let body = braced_items(input)?;
                    return Ok((Item::Card(kind, k, body), true));
                }
            }
            // `distinct(q.iter())`
            "distinct" if fork.peek(token::Paren) => {
                let inner;
                parenthesized!(inner in fork);
                if inner.parse::<Expr>().is_ok()
                    && inner.is_empty()
                    && (fork.is_empty() || fork.peek(Token![,]))
                {
                    input.parse::<Ident>()?;
                    let content;
                    parenthesized!(content in input);
                    let expr: Expr = content.parse()?;
                    return Ok((Item::Distinct(expr), false));
                }
            }
            _ => {}
        }
    }
    Ok((parse_atom(input)?, false))
}

// ==== Leaf rewriting: only the top-level comparison/logic operators become z3 operators ====

fn strip(mut expr: &Expr) -> &Expr {
    loop {
        match expr {
            Expr::Paren(p) => expr = &p.expr,
            Expr::Group(g) => expr = &g.expr,
            _ => return expr,
        }
    }
}

/// `sum(iter, |pat| body)`
fn as_sum_by(call: &ExprCall) -> Option<(&Expr, &ExprClosure)> {
    let Expr::Path(p) = &*call.func else {
        return None;
    };
    if !p.path.is_ident("sum") || call.args.len() != 2 {
        return None;
    }
    match &call.args[1] {
        Expr::Closure(cl) if matches!(cl.output, ReturnType::Default) => Some((&call.args[0], cl)),
        _ => None,
    }
}

fn as_lifted_call(call: &ExprCall) -> Option<&'static str> {
    {
        let Expr::Path(p) = &*call.func else {
            return None;
        };
        let name = match () {
            _ if p.path.is_ident("abs") && call.args.len() == 1 => "abs",
            _ if p.path.is_ident("max") && call.args.len() == 2 => "max",
            _ if p.path.is_ident("min") && call.args.len() == 2 => "min",
            _ => return None,
        };
        Some(name)
    }
}

fn is_differs(call: &ExprCall) -> bool {
    matches!(&*call.func, Expr::Path(p) if p.path.is_ident("differs")) && call.args.len() == 2
}

/// `[x, y]` → `[&(x), &(y)]`
fn borrow_elems(expr: &Expr) -> TokenStream {
    let Expr::Array(arr) = strip(expr) else {
        return quote!(#expr);
    };
    let elems = arr.elems.iter().map(|e| match strip(e) {
        Expr::Reference(_) => quote!(#e),
        _ => quote_spanned!(e.span()=> &(#e)),
    });
    quote_spanned!(arr.span()=> [#(#elems),*])
}

/// Whether the value itself is arithmetic
fn is_arith(expr: &Expr) -> bool {
    match strip(expr) {
        Expr::Lit(l) => matches!(l.lit, Lit::Int(_)),
        Expr::Unary(u) => matches!(u.op, UnOp::Neg(_)),
        Expr::Binary(b) => matches!(
            b.op,
            BinOp::Add(_) | BinOp::Sub(_) | BinOp::Mul(_) | BinOp::Div(_) | BinOp::Rem(_)
        ),
        Expr::Call(c) => as_sum_by(c).is_some() || as_lifted_call(c).is_some(),
        _ => false,
    }
}

/// Turn the left expressions into z3 value by borrowing
fn operand(expr: &Expr, arith: bool) -> TokenStream {
    match expr {
        Expr::Paren(p) => {
            let inner = operand(&p.expr, arith);
            quote_spanned!(p.span()=> (#inner))
        }
        Expr::Group(g) => operand(&g.expr, arith),
        Expr::Lit(l) if matches!(&l.lit, Lit::Int(i) if i.suffix().is_empty()) => {
            quote_spanned!(l.span()=> ::z3_toolkit::__private::int_lit(#expr))
        }
        Expr::Unary(u) if matches!(u.op, UnOp::Neg(_)) => {
            let inner = operand(&u.expr, true);
            quote_spanned!(u.span()=> -#inner)
        }
        Expr::Binary(b)
            if matches!(
                b.op,
                BinOp::Add(_) | BinOp::Sub(_) | BinOp::Mul(_) | BinOp::Div(_) | BinOp::Rem(_)
            ) =>
        {
            let (lhs, rhs) = (operand(&b.left, true), operand(&b.right, true));
            let op = &b.op;
            quote_spanned!(b.span()=> #lhs #op #rhs)
        }
        Expr::Call(c) if as_sum_by(c).is_some() => {
            let (iter, cl) = as_sum_by(c).unwrap();
            let (cap, inputs) = (&cl.capture, &cl.inputs);
            let body = operand(&cl.body, true);
            quote_spanned!(c.span()=> ::z3_toolkit::__private::sum_by(#iter, #cap |#inputs| #body))
        }
        Expr::Call(c) if as_lifted_call(c).is_some() => {
            let f = Ident::new(as_lifted_call(c).unwrap(), c.func.span());
            let args = c.args.iter().map(|e| operand(e, true));
            quote_spanned!(c.span()=> ::z3_toolkit::#f(#(#args),*))
        }
        _ if arith => quote_spanned!(expr.span()=> ::z3_toolkit::__private::Arith::arith(&(#expr))),
        _ => quote_spanned!(expr.span()=> ::z3_toolkit::__private::Operand::operand(&(#expr))),
    }
}

fn compare(bin_expr: &ExprBinary) -> Option<TokenStream> {
    let (method, ordering) = match bin_expr.op {
        BinOp::Lt(_) => ("lt", true),
        BinOp::Le(_) => ("le", true),
        BinOp::Gt(_) => ("gt", true),
        BinOp::Ge(_) => ("ge", true),
        BinOp::Eq(_) => ("eq", false),
        BinOp::Ne(_) => ("ne", false),
        _ => return None,
    };
    // Ordering cmp and cmp containing arithmetic treat `Bool` as 0/1
    let arith = ordering || is_arith(&bin_expr.left) || is_arith(&bin_expr.right);
    let (lhs, rhs) = (
        operand(&bin_expr.left, arith),
        operand(&bin_expr.right, arith),
    );
    let m = Ident::new(method, bin_expr.op.span());
    Some(quote_spanned!(bin_expr.span()=> (#lhs).#m(#rhs)))
}

/// `lhs in rhs`: two cmp for a range, the disjunction of `lhs == val` for an array or iterator
fn member(lhs: &Expr, rhs: &Expr) -> TokenStream {
    let lhs = operand(lhs, true);
    let val = Ident::new("__z3_in", Span::mixed_site());
    match strip(rhs) {
        Expr::Range(r) => {
            let mut parts = Vec::new();
            if let Some(s) = &r.start {
                let s = operand(s, true);
                parts.push(quote!(#val.ge(#s)));
            }
            if let Some(e) = &r.end {
                let e = operand(e, true);
                parts.push(match r.limits {
                    RangeLimits::Closed(_) => quote!(#val.le(#e)),
                    RangeLimits::HalfOpen(_) => quote!(#val.lt(#e)),
                });
            }
            quote_spanned!(rhs.span()=> {
                let #val = #lhs;
                ::z3_toolkit::__private::and(::std::vec![#(#parts),*])
            })
        }
        Expr::Array(arr) => {
            let elems = arr.elems.iter().map(|e| operand(e, true));
            quote_spanned!(rhs.span()=> ::z3_toolkit::__private::member(&(#lhs), [#(#elems), *]))
        }
        _ => quote_spanned!(rhs.span()=> ::z3_toolkit::__private::member(&(#lhs), #rhs)),
    }
}

fn leaf(expr: &Expr) -> TokenStream {
    match expr {
        Expr::Paren(p) => leaf(&p.expr),
        Expr::Group(g) => leaf(&g.expr),
        Expr::Unary(u) if matches!(u.op, UnOp::Not(_)) => {
            let inner = leaf(&u.expr);
            quote_spanned!(u.span()=> ::z3_toolkit::__private::not(&#inner))
        }
        Expr::Binary(b) => match b.op {
            BinOp::And(_) => {
                let (lhs, rhs) = (leaf(&b.left), leaf(&b.right));
                quote_spanned!(b.span() => ::z3_toolkit::__private::and(::std::vec![#lhs, #rhs]))
            }
            BinOp::Or(_) => {
                let (lhs, rhs) = (leaf(&b.left), leaf(&b.right));
                quote_spanned!(b.span() => ::z3_toolkit::__private::or(::std::vec![#lhs, #rhs]))
            }
            _ => match compare(b) {
                Some(t) => t,
                None => passthrough(expr),
            },
        },
        Expr::Call(c) if is_differs(c) => {
            let (model, vars) = (&c.args[0], borrow_elems(&c.args[1]));
            quote_spanned!(c.span()=> ::z3_toolkit::differs(#model, #vars))
        }
        _ => passthrough(expr),
    }
}

fn passthrough(expr: &Expr) -> TokenStream {
    quote_spanned!(expr.span()=> ::z3_toolkit::__private::ToBool::to_bool(&(#expr)))
}

fn leaf_bool(item: &Item) -> TokenStream {
    match item {
        Item::Leaf(e) => leaf(e),
        Item::Member(lhs, rhs) => member(lhs, rhs),
        _ => unreachable!("labels only hold leaves"),
    }
}

// ==== Code generation ====================

struct Builder {
    n: usize,
}

impl Builder {
    fn ident(&mut self) -> Ident {
        self.n += 1;
        Ident::new(&format!("__z3_children_{}", self.n), Span::mixed_site())
    }

    fn items(&mut self, items: &[Item], out: &Ident, labels: Option<&Ident>) -> TokenStream {
        let parts: Vec<TokenStream> = items.iter().map(|i| self.item(i, out, labels)).collect();
        quote!(#(#parts)*)
    }

    fn collect(&mut self, items: &[Item]) -> (Ident, TokenStream) {
        let v = self.ident();
        let body = self.items(items, &v, None);
        (v, body)
    }

    fn item(&mut self, item: &Item, out: &Ident, labels: Option<&Ident>) -> TokenStream {
        match item {
            Item::Leaf(_) | Item::Member(..) => {
                let b = leaf_bool(item);
                quote!(#out.push(#b);)
            }
            Item::Label(lit, inner) => match labels {
                Some(l) => {
                    let (v, body) = self.collect(std::slice::from_ref(&**inner));
                    quote!({
                        #[allow(unused_mut)]
                        let mut #v = ::std::vec::Vec::<::z3_toolkit::__private::Bool>::new();
                        #body
                        #l.push((
                            ::std::string::String::from(#lit),
                            ::z3_toolkit::__private::and(#v),
                        ));
                    })
                }
                None => syn::Error::new(
                    lit.span(),
                    "labels are only allowed at the top level of `constraints!` \
                    (inside `for` / `if` is fine)
                ",
                )
                .to_compile_error(),
            },
            Item::Group(kind, body) => {
                let (v, inner) = self.collect(body);
                let call = match kind {
                    GroupKind::All => quote!(::z3_toolkit::__private::and(#v)),
                    GroupKind::Any => quote!(::z3_toolkit::__private::or(#v)),
                    GroupKind::Not => {
                        quote!(::z3_toolkit::__private::not(&::z3_toolkit::__private::and(#v)))
                    }
                };
                quote!({
                    #[allow(unused_mut)]
                    let mut #v = ::std::vec::Vec::<::z3_toolkit::__private::Bool>::new();
                    #inner
                    #out.push(#call);
                })
            }
            Item::Implies(lhs, rhs) => {
                let (lv, li) = self.collect(lhs);
                let (rv, ri) = self.collect(rhs);
                quote!({
                    #[allow(unused_mut)]
                    let mut #lv = ::std::vec::Vec::<::z3_toolkit::__private::Bool>::new();
                    #li
                    #[allow(unused_mut)]
                    let mut #rv = ::std::vec::Vec::<::z3_toolkit::__private::Bool>::new();
                    #ri
                    #out.push(::z3_toolkit::__private::implies(
                        &::z3_toolkit::__private::and(#lv),
                        &::z3_toolkit::__private::and(#rv),
                    ));
                })
            }
            Item::Card(kind, k, body) => {
                let (v, inner) = self.collect(body);
                let f = match kind {
                    CardKind::Exactly => quote!(exactly),
                    CardKind::AtMost => quote!(at_most),
                    CardKind::AtLeast => quote!(at_least),
                };
                quote!({
                    #[allow(unused_mut)]
                    let mut #v = ::std::vec::Vec::<::z3_toolkit::__private::Bool>::new();
                    #inner
                    #out.push(::z3_toolkit::__private::#f(&#v, (#k) as i32));
                })
            }
            Item::Distinct(e) => {
                quote!(#out.push(::z3_toolkit::__private::distinct(#e));)
            }
            Item::For(pat, iter, body) => {
                let inner = self.items(body, out, labels);
                quote!(for #pat in #iter { #inner })
            }
            Item::If(cond, then, els) => {
                let t = self.items(then, out, labels);
                match els {
                    Some(e) => {
                        let e = self.items(e, out, labels);
                        quote!(if #cond { #t } else { #e })
                    }
                    None => quote!(if #cond { #t }),
                }
            }
        }
    }
}

// ==== Entry point ====================

pub fn constraint(input: TokenStream) -> Result<TokenStream> {
    let items = Parser::parse2(|i: ParseStream| parse_items(i), input)?;
    let mut b = Builder { n: 0 };
    let out = b.ident();
    let body = b.items(&items, &out, None);
    Ok(quote!({
        #[allow(unused_mut)]
        let mut #out = ::std::vec::Vec::<::z3_toolkit::__private::Bool>::new();
        #body
        ::z3_toolkit::__private::and(#out)
    }))
}

pub fn constraints(input: TokenStream) -> Result<TokenStream> {
    let (tgt, items) = Parser::parse2(
        |i: ParseStream| {
            let tgt: Expr = i.parse()?;
            i.parse::<Token![;]>().map_err(|e| {
                syn::Error::new(
                    e.span(),
                    "expected `;` after the target: `constraints! {solver; .. }`",
                )
            })?;
            let items = parse_items(i)?;
            Ok((tgt, items))
        },
        input,
    )?;
    let mut b = Builder { n: 0 };
    let (t, out, lab, n, c) = (b.ident(), b.ident(), b.ident(), b.ident(), b.ident());
    let body = b.items(&items, &out, Some(&lab));
    Ok(quote!({
        let #t = &(#tgt);
        #[allow(unused_mut)]
        let mut #out = ::std::vec::Vec::<::z3_toolkit::__private::Bool>::new();
        #[allow(unused_mut)]
        let mut #lab = ::std::vec::Vec::<(::std::string::String, ::z3_toolkit::__private::Bool)>::new();
        #body
        ::z3_toolkit::Engine::assert_all(#t, #out);
        for (#n, #c) in #lab {
            ::z3_toolkit::Engine::assert_labeled(#t, &#n, &#c);
        }
    }))
}

pub fn lin(input: TokenStream) -> Result<TokenStream> {
    let e: Expr = parse2(input)?;
    Ok(operand(&e, true))
}
