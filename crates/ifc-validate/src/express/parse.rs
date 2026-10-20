//! Reading EXPRESS declarations (ISO 10303-11 clause 9) into [`ast`].
//!
//! A recursive-descent reader over [`lexer`] tokens. It reads the whole
//! schema, not only the rules: an entity's explicit attributes give the
//! positional slots a rule's `SELF.Name` reads, and their types say how a
//! file value is typed (`TYPEOF`) and indexed (an `ARRAY [0:n]`).
//!
//! Expressions and statements are in [`super::parse_expr`] and
//! [`super::parse_stmt`].

use std::sync::Arc;

use super::ast::{
    AggKind, DomainRule, EntityDecl, Explicit, Inverse, Name, SchemaDecl, Simple, TypeDecl,
    TypeExpr,
};
use super::error::{ExpressError, SourcePos};
use super::lexer::{tokenize, Spanned, Tok};

/// The token cursor shared by the declaration, statement and expression
/// readers.
pub(crate) struct Parser {
    toks: Vec<Spanned>,
    at: usize,
}

impl Parser {
    pub(crate) fn new(source: &str) -> Result<Self, ExpressError> {
        Ok(Self {
            toks: tokenize(source)?,
            at: 0,
        })
    }

    pub(crate) fn peek(&self) -> &Tok {
        &self.toks[self.at].tok
    }

    pub(crate) fn peek_at(&self, ahead: usize) -> &Tok {
        let index = (self.at + ahead).min(self.toks.len() - 1);
        &self.toks[index].tok
    }

    pub(crate) fn pos(&self) -> SourcePos {
        self.toks[self.at].pos
    }

    pub(crate) fn bump(&mut self) -> Tok {
        let tok = self.toks[self.at].tok.clone();
        if self.at + 1 < self.toks.len() {
            self.at += 1;
        }
        tok
    }

    pub(crate) fn error(&self, message: impl Into<String>) -> ExpressError {
        ExpressError::syntax(self.pos(), message)
    }

    /// Whether the next token is the keyword `word` (upper case).
    pub(crate) fn is_word(&self, word: &str) -> bool {
        matches!(self.peek(), Tok::Word(upper, _) if &**upper == word)
    }

    pub(crate) fn eat_word(&mut self, word: &str) -> bool {
        let found = self.is_word(word);
        if found {
            self.bump();
        }
        found
    }

    pub(crate) fn expect_word(&mut self, word: &str) -> Result<(), ExpressError> {
        if self.eat_word(word) {
            Ok(())
        } else {
            Err(self.error(format!("expected {word}, found {}", describe(self.peek()))))
        }
    }

    pub(crate) fn is_sym(&self, symbol: &str) -> bool {
        matches!(self.peek(), Tok::Sym(found) if *found == symbol)
    }

    pub(crate) fn eat_sym(&mut self, symbol: &str) -> bool {
        let found = self.is_sym(symbol);
        if found {
            self.bump();
        }
        found
    }

    pub(crate) fn expect_sym(&mut self, symbol: &str) -> Result<(), ExpressError> {
        if self.eat_sym(symbol) {
            Ok(())
        } else {
            Err(self.error(format!(
                "expected `{symbol}`, found {}",
                describe(self.peek())
            )))
        }
    }

    /// An identifier: upper-cased name and spelling.
    pub(crate) fn ident(&mut self) -> Result<(Name, Arc<str>), ExpressError> {
        match self.bump() {
            Tok::Word(upper, spelling) => Ok((upper, spelling)),
            other => Err(self.error(format!(
                "expected an identifier, found {}",
                describe(&other)
            ))),
        }
    }

    /// Skips to just past the next `;` at bracket depth zero.
    pub(crate) fn skip_statement(&mut self) -> Result<(), ExpressError> {
        let mut depth = 0i32;
        loop {
            match self.bump() {
                Tok::End => return Err(self.error("unexpected end of source")),
                Tok::Sym("(" | "[" | "{") => depth += 1,
                Tok::Sym(")" | "]" | "}") => depth -= 1,
                Tok::Sym(";") if depth <= 0 => return Ok(()),
                _ => {}
            }
        }
    }
}

/// A token, for messages.
pub(crate) fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Word(_, spelling) => format!("`{spelling}`"),
        Tok::Int(value) => value.to_string(),
        Tok::Real(value) => value.to_string(),
        Tok::Str(text) => format!("'{text}'"),
        Tok::Binary(bits) => format!("%{bits}"),
        Tok::Sym(symbol) => format!("`{symbol}`"),
        Tok::End => "the end of the source".to_owned(),
    }
}

/// Reads a whole schema.
pub(crate) fn schema(source: &str) -> Result<SchemaDecl, ExpressError> {
    let mut p = Parser::new(source)?;
    p.expect_word("SCHEMA")?;
    let (_, name) = p.ident()?;
    // An optional schema version id: `SCHEMA x 'id';`.
    if let Tok::Str(_) = p.peek() {
        p.bump();
    }
    p.expect_sym(";")?;
    let mut out = SchemaDecl {
        name,
        ..SchemaDecl::default()
    };
    loop {
        let word = match p.peek() {
            Tok::Word(upper, _) => upper.clone(),
            other => {
                return Err(p.error(format!("expected a declaration, found {}", describe(other))))
            }
        };
        match &*word {
            "END_SCHEMA" => {
                p.bump();
                p.eat_sym(";");
                return Ok(out);
            }
            "TYPE" => out.types.push(type_decl(&mut p)?),
            "ENTITY" => out.entities.push(entity_decl(&mut p)?),
            "FUNCTION" => out.functions.push(super::parse_stmt::function(&mut p)?),
            "RULE" => out.rules.push(super::parse_stmt::rule(&mut p)?),
            "USE" | "REFERENCE" => p.skip_statement()?,
            other => {
                return Err(p.error(format!(
                    "`{other}` declarations are not read by this evaluator"
                )))
            }
        }
    }
}

fn type_decl(p: &mut Parser) -> Result<TypeDecl, ExpressError> {
    p.expect_word("TYPE")?;
    let (name, spelling) = p.ident()?;
    p.expect_sym("=")?;
    p.eat_word("EXTENSIBLE");
    p.eat_word("GENERIC_ENTITY");
    let underlying = if p.eat_word("ENUMERATION") {
        p.expect_word("OF")?;
        TypeExpr::Enumeration(name_list(p)?)
    } else if p.eat_word("SELECT") {
        TypeExpr::Select(name_list(p)?)
    } else {
        type_expr(p)?
    };
    p.expect_sym(";")?;
    let rules = if p.eat_word("WHERE") {
        domain_rules(p, &["END_TYPE"])?
    } else {
        Vec::new()
    };
    p.expect_word("END_TYPE")?;
    p.expect_sym(";")?;
    Ok(TypeDecl {
        name,
        spelling,
        underlying,
        rules,
    })
}

/// `( a, b, c )`.
fn name_list(p: &mut Parser) -> Result<Vec<Name>, ExpressError> {
    p.expect_sym("(")?;
    let mut names = Vec::new();
    loop {
        names.push(p.ident()?.0);
        if !p.eat_sym(",") {
            break;
        }
    }
    p.expect_sym(")")?;
    Ok(names)
}

/// A type reference: simple, named, aggregate or generic.
pub(crate) fn type_expr(p: &mut Parser) -> Result<TypeExpr, ExpressError> {
    let (word, _) = p.ident()?;
    let kind = match &*word {
        "LIST" => Some(AggKind::List),
        "SET" => Some(AggKind::Set),
        "BAG" => Some(AggKind::Bag),
        "ARRAY" => Some(AggKind::Array),
        _ => None,
    };
    if let Some(kind) = kind {
        let mut lower = None;
        if p.eat_sym("[") {
            lower = Some(Box::new(super::parse_expr::expression(p)?));
            p.expect_sym(":")?;
            super::parse_expr::expression(p)?;
            p.expect_sym("]")?;
        }
        p.expect_word("OF")?;
        p.eat_word("OPTIONAL");
        p.eat_word("UNIQUE");
        let elem = Box::new(type_expr(p)?);
        return Ok(TypeExpr::Aggregate { kind, lower, elem });
    }
    let simple = match &*word {
        "INTEGER" => Some(Simple::Integer),
        "REAL" => Some(Simple::Real),
        "NUMBER" => Some(Simple::Number),
        "BOOLEAN" => Some(Simple::Boolean),
        "LOGICAL" => Some(Simple::Logical),
        "STRING" => Some(Simple::String),
        "BINARY" => Some(Simple::Binary),
        _ => None,
    };
    if let Some(simple) = simple {
        // Width or precision: `STRING(255) FIXED`, `REAL(15)`.
        if p.eat_sym("(") {
            super::parse_expr::expression(p)?;
            p.expect_sym(")")?;
        }
        p.eat_word("FIXED");
        return Ok(TypeExpr::Simple(simple));
    }
    match &*word {
        "GENERIC" | "GENERIC_ENTITY" => {
            if p.eat_sym(":") {
                p.ident()?;
            }
            Ok(TypeExpr::Generic)
        }
        "AGGREGATE" => {
            if p.eat_sym(":") {
                p.ident()?;
            }
            p.expect_word("OF")?;
            type_expr(p)?;
            Ok(TypeExpr::Generic)
        }
        _ => Ok(TypeExpr::Named(word)),
    }
}

/// Labelled domain rules up to (not including) one of `ends`.
pub(crate) fn domain_rules(p: &mut Parser, ends: &[&str]) -> Result<Vec<DomainRule>, ExpressError> {
    let mut rules = Vec::new();
    let mut unlabelled = 0;
    while !ends.iter().any(|end| p.is_word(end)) {
        let label = if matches!(p.peek(), Tok::Word(..)) && matches!(p.peek_at(1), Tok::Sym(":")) {
            let (_, spelling) = p.ident()?;
            p.bump();
            spelling
        } else {
            unlabelled += 1;
            Arc::from(format!("WR{unlabelled}"))
        };
        let expr = super::parse_expr::expression(p)?;
        p.expect_sym(";")?;
        rules.push(DomainRule {
            label,
            expr,
            frame: 0,
        });
    }
    Ok(rules)
}

fn entity_decl(p: &mut Parser) -> Result<EntityDecl, ExpressError> {
    p.expect_word("ENTITY")?;
    let (name, spelling) = p.ident()?;
    let mut supertypes = Vec::new();
    while !p.eat_sym(";") {
        if p.eat_word("SUBTYPE") {
            p.expect_word("OF")?;
            supertypes = name_list(p)?;
        } else if p.eat_word("SUPERTYPE") {
            if p.eat_word("OF") {
                skip_parenthesised(p)?;
            }
        } else if p.eat_word("ABSTRACT") {
        } else {
            return Err(p.error(format!(
                "unexpected {} in an entity head",
                describe(p.peek())
            )));
        }
    }
    let mut entity = EntityDecl {
        name,
        spelling,
        supertypes,
        explicit: Vec::new(),
        derived: Vec::new(),
        inverses: Vec::new(),
        rules: Vec::new(),
    };
    let ends = ["DERIVE", "INVERSE", "UNIQUE", "WHERE", "END_ENTITY"];
    while !ends.iter().any(|end| p.is_word(end)) {
        explicit_attributes(p, &mut entity.explicit)?;
    }
    if p.eat_word("DERIVE") {
        while !ends[1..].iter().any(|end| p.is_word(end)) {
            let (name, spelling, _) = attribute_name(p)?;
            p.expect_sym(":")?;
            type_expr(p)?;
            p.expect_sym(":=")?;
            let expr = super::parse_expr::expression(p)?;
            p.expect_sym(";")?;
            entity.derived.push(super::ast::Derived {
                name,
                spelling,
                expr,
                frame: 0,
            });
        }
    }
    if p.eat_word("INVERSE") {
        while !ends[2..].iter().any(|end| p.is_word(end)) {
            entity.inverses.push(inverse(p)?);
        }
    }
    if p.eat_word("UNIQUE") {
        while !ends[3..].iter().any(|end| p.is_word(end)) {
            p.skip_statement()?;
        }
    }
    if p.eat_word("WHERE") {
        entity.rules = domain_rules(p, &["END_ENTITY"])?;
    }
    p.expect_word("END_ENTITY")?;
    p.expect_sym(";")?;
    Ok(entity)
}

/// `a, b : OPTIONAL T;`, each name its own attribute.
fn explicit_attributes(p: &mut Parser, out: &mut Vec<Explicit>) -> Result<(), ExpressError> {
    let mut names = vec![attribute_name(p)?];
    while p.eat_sym(",") {
        names.push(attribute_name(p)?);
    }
    p.expect_sym(":")?;
    p.eat_word("OPTIONAL");
    let ty = type_expr(p)?;
    p.expect_sym(";")?;
    out.extend(
        names
            .into_iter()
            .map(|(name, spelling, redeclares)| Explicit {
                name,
                spelling,
                ty: ty.clone(),
                redeclares,
            }),
    );
    Ok(())
}

/// `Name`, or `SELF\Supertype.Name` (a redeclaration, naming the
/// supertype).
fn attribute_name(p: &mut Parser) -> Result<(Name, Arc<str>, Option<Name>), ExpressError> {
    if p.eat_word("SELF") {
        p.expect_sym("\\")?;
        let (supertype, _) = p.ident()?;
        p.expect_sym(".")?;
        let (name, spelling) = p.ident()?;
        // `RENAMED x` gives the redeclared attribute a new name.
        if p.eat_word("RENAMED") {
            let (renamed, renamed_spelling) = p.ident()?;
            return Ok((renamed, renamed_spelling, Some(supertype)));
        }
        return Ok((name, spelling, Some(supertype)));
    }
    let (name, spelling) = p.ident()?;
    Ok((name, spelling, None))
}

/// `Name : [SET|BAG [l:u] OF] Entity FOR [Entity.]Attribute;`
fn inverse(p: &mut Parser) -> Result<Inverse, ExpressError> {
    let (name, _, _) = attribute_name(p)?;
    p.expect_sym(":")?;
    let mut aggregate = None;
    let kind = if p.eat_word("SET") {
        Some(AggKind::Set)
    } else if p.eat_word("BAG") {
        Some(AggKind::Bag)
    } else {
        None
    };
    if let Some(kind) = kind {
        let (mut lower, mut upper) = (0, None);
        if p.eat_sym("[") {
            lower = bound(p)?.unwrap_or(0);
            p.expect_sym(":")?;
            upper = bound(p)?;
            p.expect_sym("]")?;
        }
        p.expect_word("OF")?;
        aggregate = Some((kind, lower, upper));
    }
    let (entity, _) = p.ident()?;
    p.expect_word("FOR")?;
    let (mut for_attribute, _) = p.ident()?;
    if p.eat_sym(".") {
        for_attribute = p.ident()?.0;
    }
    p.expect_sym(";")?;
    Ok(Inverse {
        name,
        entity,
        for_attribute,
        aggregate,
    })
}

/// An integer bound, or `None` for `?`.
fn bound(p: &mut Parser) -> Result<Option<u64>, ExpressError> {
    match p.bump() {
        Tok::Int(value) => Ok(Some(u64::try_from(value).unwrap_or(0))),
        Tok::Sym("?") => Ok(None),
        other => Err(p.error(format!("expected a bound, found {}", describe(&other)))),
    }
}

/// Skips a balanced `( ... )`.
fn skip_parenthesised(p: &mut Parser) -> Result<(), ExpressError> {
    p.expect_sym("(")?;
    let mut depth = 1;
    while depth > 0 {
        match p.bump() {
            Tok::Sym("(") => depth += 1,
            Tok::Sym(")") => depth -= 1,
            Tok::End => return Err(p.error("unbalanced parentheses")),
            _ => {}
        }
    }
    Ok(())
}
