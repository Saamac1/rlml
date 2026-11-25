//! RogueScript syntax tree (RS 12). `Display` writes a node back as
//! RogueScript, with fully parenthesized operators, so tests can compare
//! the parse of an expression with what was meant.

use std::fmt;

/// Line and 1-based column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Dice(u32, u32),
    /// Milliseconds and the text as written (`0500ms`, `1s`).
    Time(u64, String),
    Percent(i64),
    Size(u32, u32),
    Str(String),
    Bool(bool),
    None,
    All,
    /// An upper-case name, dotted parts joined (`LOOT.RARE`); `root` for `.LOOT`.
    Name {
        name: String,
        root: bool,
    },
    /// A lower-case word: local, context or engine name, direction.
    Word(String),
    /// `ITEM.*` or `ITEM.**`.
    Wild {
        name: String,
        root: bool,
        deep: bool,
    },
    List(Vec<Expr>),
    /// `{}`.
    EmptyDict,
    /// `e.field`, `e.HP`.
    Field(Box<Expr>, String),
    /// A tile index `GLYPH.NKI.1`.
    TileIndex(Box<Expr>, i64),
    Call(Box<Expr>, Vec<Arg>),
    Subscript(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `a..b`, or `a..` when open.
    Range(Box<Expr>, Option<Box<Expr>>),
    /// `e has [no] COMPONENT`.
    Has {
        subject: Box<Expr>,
        no: bool,
        component: String,
    },
    Is(Box<Expr>, Box<Expr>),
    In(Box<Expr>, Box<Expr>),
    Where(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl BinOp {
    pub fn text(self) -> &'static str {
        match self {
            BinOp::Or => "or",
            BinOp::And => "and",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
        }
    }
}

/// `value` or `name: value`.
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub pos: Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    /// `name = wait number(…)` or `name = wait text(…)`.
    WaitInput {
        name: String,
        kind: String,
        arg: Expr,
    },
    /// `name = open MENU(…)`.
    OpenValue {
        name: String,
        call: Expr,
    },
    If {
        arms: Vec<(Expr, Vec<Stmt>)>,
        otherwise: Option<Vec<Stmt>>,
    },
    For {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    Repeat {
        count: Expr,
        body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Break,
    /// Cases; `None` is `else`.
    Match {
        subject: Expr,
        cases: Vec<(Option<Expr>, Vec<Stmt>)>,
    },
    Choice(Vec<ChoiceOption>),
    Emit(Expr),
    Wait(WaitArg),
    Open(Expr),
    Return(Option<Expr>),
    Cancel,
    Call(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WaitArg {
    Time(Expr),
    Confirm,
    Scene(Expr),
    Input { kind: String, arg: Expr },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceOption {
    pub name: String,
    pub args: Vec<Arg>,
    pub hide: Option<Expr>,
    pub disabled: Option<Expr>,
    pub body: OptionBody,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionBody {
    Block(Vec<Stmt>),
    Adjust {
        target: Expr,
        range: Expr,
        pool: Option<Expr>,
    },
}

/// A type as written (RS 12.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    /// `number`, `list`, `dict`, `collection`, … or a component NAME.
    pub base: String,
    /// `list T`, `dict K V`: the member, key and value types.
    pub args: Vec<Type>,
    /// Words after the base that are not types: `copy`, `card`, `26`,
    /// `merge STACKABLE`, `sort score desc`.
    pub extra: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    From(Expr),
    Aim { range: Expr, at: Option<Expr> },
    Ask(Option<Expr>),
    Count,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub default: Option<Expr>,
    pub source: Option<Source>,
    pub pos: Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Before,
    On,
    After,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Handler {
    pub phase: Phase,
    /// Upper-case event (`HIT`) or engine event (`turn`).
    pub event: String,
    pub names: Vec<String>,
    pub name: Option<String>,
    pub priority: i64,
    pub when: Option<Expr>,
    pub body: Vec<Stmt>,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReturnType {
    Type(Type),
    /// `name|text`, for the naming query (RS 12.1).
    NameOrText,
}

/// A timeline's time: `0500ms`, `+200ms`, `OPEN+1s`, `12` (RS 12.4).
#[derive(Debug, Clone, PartialEq)]
pub enum Time {
    At(Expr),
    After(Expr),
    Mark {
        name: String,
        offset: Option<(bool, Expr)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Action {
        time: Option<Time>,
        span: Option<Expr>,
        always: bool,
        when: Option<Expr>,
        action: Action,
        pos: Pos,
    },
    When {
        time: Option<Time>,
        cond: Expr,
        rows: Vec<Row>,
        otherwise: Vec<Row>,
        pos: Pos,
    },
    Mark {
        time: Time,
        name: String,
        pos: Pos,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Call(Expr),
    Wait(WaitArg),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub name: String,
    pub looped: Option<Expr>,
    pub rows: Vec<Row>,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    pub background: bool,
    pub realtime: bool,
    pub tracks: Vec<Track>,
    /// Rows outside tracks.
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Component {
        fields: Vec<Param>,
        handlers: Vec<Handler>,
    },
    Event {
        params: Vec<Param>,
    },
    Handler(Handler),
    Function {
        present: bool,
        query: bool,
        params: Vec<Param>,
        ret: Option<ReturnType>,
        body: Vec<Stmt>,
    },
    /// `noverb` hides the command from the verb coin (RS 7.1).
    Command {
        params: Vec<Param>,
        cost: Option<i64>,
        noverb: bool,
        body: Vec<Stmt>,
    },
    /// `command NAME = CALL`.
    CommandAlias(Expr),
    Ai {
        body: Vec<Stmt>,
    },
    Generator {
        params: Vec<Param>,
        size: Option<(u32, u32)>,
        body: Vec<Stmt>,
    },
    Map {
        transient: bool,
        wrap: bool,
        nosave: bool,
        generate: Expr,
    },
    Scene {
        params: Vec<Param>,
        body: Vec<Stmt>,
    },
    Timeline {
        params: Vec<Param>,
        timeline: Timeline,
    },
}

/// A top-level declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The written name; empty for an unnamed handler.
    pub name: String,
    pub overrides: bool,
    pub kind: ItemKind,
    pub pos: Pos,
}

fn list<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T]) -> fmt::Result {
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{x}")?;
    }
    Ok(())
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ExprKind::*;
        let dot = |root: bool| if root { "." } else { "" };
        match &self.kind {
            Int(n) => write!(f, "{n}"),
            Dice(n, s) => write!(f, "{n}d{s}"),
            Time(_, t) => f.write_str(t),
            Percent(p) => write!(f, "{p}%"),
            Size(w, h) => write!(f, "{w}x{h}"),
            Str(s) => write!(f, "\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Bool(b) => write!(f, "{b}"),
            None => f.write_str("none"),
            All => f.write_str("all"),
            Name { name, root } => write!(f, "{}{name}", dot(*root)),
            Word(w) => f.write_str(w),
            Wild { name, root, deep } => {
                write!(f, "{}{name}.{}", dot(*root), if *deep { "**" } else { "*" })
            }
            List(items) => {
                f.write_str("[")?;
                list(f, items)?;
                f.write_str("]")
            }
            EmptyDict => f.write_str("{}"),
            Field(e, n) => write!(f, "{e}.{n}"),
            TileIndex(e, i) => write!(f, "{e}.{i}"),
            Call(e, args) => {
                write!(f, "{e}(")?;
                list(f, args)?;
                f.write_str(")")
            }
            Subscript(e, i) => write!(f, "{e}[{i}]"),
            Neg(e) => write!(f, "-{e}"),
            Not(e) => write!(f, "(not {e})"),
            Binary(op, a, b) => write!(f, "({a} {} {b})", op.text()),
            Range(a, Some(b)) => write!(f, "({a}..{b})"),
            Range(a, Option::None) => write!(f, "({a}..)"),
            Has {
                subject,
                no,
                component,
            } => write!(
                f,
                "({subject} has {}{component})",
                if *no { "no " } else { "" }
            ),
            Is(a, b) => write!(f, "({a} is {b})"),
            In(a, b) => write!(f, "({a} in {b})"),
            Where(a, b) => write!(f, "({a} where {b})"),
        }
    }
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(n) => write!(f, "{n}: {}", self.value),
            None => write!(f, "{}", self.value),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.base)?;
        for a in &self.args {
            write!(f, " {a}")?;
        }
        for e in &self.extra {
            write!(f, " {e}")?;
        }
        Ok(())
    }
}

impl fmt::Display for ReturnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReturnType::Type(t) => write!(f, "{t}"),
            ReturnType::NameOrText => f.write_str("name|text"),
        }
    }
}
