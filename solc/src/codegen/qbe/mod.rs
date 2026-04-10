use std::{borrow::Cow, rc::Rc};

pub mod build;
pub mod fmt;
pub mod lower;

pub type Alignment = usize;
pub type Size = usize;
pub type Offset = usize;

#[derive(Debug, PartialEq, Eq)]
pub enum Ident<'a> {
    Ty(Cow<'a, str>),
    Global(Cow<'a, str>),
    Temp(Cow<'a, str>),
    Block(Cow<'a, str>),
}

impl<'a> Ident<'a> {
    pub fn ty(value: impl Into<Cow<'a, str>>) -> Self {
        Self::Ty(value.into())
    }

    pub fn global(value: impl Into<Cow<'a, str>>) -> Self {
        Self::Global(value.into())
    }

    pub fn temp(value: impl Into<Cow<'a, str>>) -> Self {
        Self::Temp(value.into())
    }

    pub fn block(value: impl Into<Cow<'a, str>>) -> Self {
        Self::Block(value.into())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Ident::Ty(cow) => cow,
            Ident::Global(cow) => cow,
            Ident::Temp(cow) => cow,
            Ident::Block(cow) => cow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseTy {
    /// 32-bit int
    Word,
    /// 64-bit int
    Long,
    /// 32-bit float
    Single,
    /// 64-bit float
    Double,
}

impl BaseTy {
    /// size in bytes
    pub fn size(&self) -> u64 {
        match self {
            Self::Word | Self::Single => 4,
            Self::Long | Self::Double => 8,
        }
    }
}

#[derive(Debug)]
pub enum ExtTy {
    /// base type
    Base(BaseTy),
    /// 8-bit int
    Byte,
    /// 16-bit int
    HalfWord,
}

impl ExtTy {
    /// size in bytes
    pub fn size(&self) -> u64 {
        match self {
            Self::Base(base_ty) => base_ty.size(),
            Self::Byte => 1,
            Self::HalfWord => 2,
        }
    }
}

#[derive(Debug)]
pub enum SubWordTy {
    SignedByte,
    UnsignedByte,
    SingedHalfWord,
    UnsingedHalfWord,
}

impl SubWordTy {
    /// size in bytes
    pub fn size(&self) -> u64 {
        match self {
            Self::SignedByte | Self::UnsignedByte => 1,
            Self::SingedHalfWord | Self::UnsingedHalfWord => 2,
        }
    }
}

#[derive(Debug)]
pub enum AbiTy<'a> {
    Base(BaseTy),
    SubWord(SubWordTy),
    Aggregate(Rc<TyDef<'a>>),
}

impl AbiTy<'_> {
    /// size in bytes
    pub fn size(&self) -> u64 {
        todo!()
    }

    pub fn into_base(&self) -> BaseTy {
        match self {
            AbiTy::Base(base_ty) => *base_ty,
            AbiTy::SubWord(_) => BaseTy::Word,
            AbiTy::Aggregate(_) => BaseTy::Long, // typedefs as ptr?
        }
    }
}

#[derive(Debug)]
pub enum SubTyKind<'a> {
    Extended(ExtTy),
    Ident(&'a str),
}

#[derive(Debug)]
pub struct SubTy<'a> {
    pub kind: SubTyKind<'a>,
    pub align: Option<Alignment>,
}

#[derive(Debug)]
pub enum TyDef<'a> {
    Regular {
        ident: Ident<'a>,
        align: Option<Alignment>,
        sub_tys: Vec<SubTy<'a>>,
    },
    Union {
        ident: Ident<'a>,
        align: Option<Alignment>,
        variants: Vec<Vec<SubTy<'a>>>,
    },
    Opaque {
        ident: Ident<'a>,
        align: Alignment,
        size: Size,
    },
}

impl TyDef<'_> {
    fn ident(&self) -> &Ident<'_> {
        match self {
            TyDef::Regular { ident, .. } => ident,
            TyDef::Union { ident, .. } => ident,
            TyDef::Opaque { ident, .. } => ident,
        }
    }

    pub fn align(&self) -> Option<Alignment> {
        match self {
            TyDef::Regular { align, .. } => *align,
            TyDef::Union { align, .. } => *align,
            TyDef::Opaque { align, .. } => Some(*align),
        }
    }
}

// LINKAGE :=
//     'export' [NL]
//   | 'thread' [NL]
//   | 'section' SECNAME [NL]
//   | 'section' SECNAME SECFLAGS [NL]
//
// SECNAME  := '"' .... '"'
// SECFLAGS := '"' .... '"'
#[derive(Debug)]
pub enum Linkage {
    Export,
    Thread,
    // SecName,
    // SecFlags,
}

#[derive(Debug)]
pub struct RegularParam<'a>(pub AbiTy<'a>, pub Operand<'a>);

#[derive(Debug)]
pub enum Param<'a> {
    Regular(RegularParam<'a>),
    Env(Ident<'a>),
    VariadicMarker,
}

#[derive(Debug)]
pub enum Sign {
    Minus,
    None,
}

#[derive(Debug)]
pub enum Precision {
    Single,
    Double,
}

// CONST :=
//     ['-'] NUMBER  # Decimal integer
//   | 's_' FP       # Single-precision float
//   | 'd_' FP       # Double-precision float
//   | $IDENT        # Global symbol
#[derive(Debug)]
pub enum Const<'a> {
    Int(Sign, i128),
    Float(Precision, f64),
    Ident(Ident<'a>),
}

impl Const<'_> {
    pub fn int(val: i128) -> Self {
        Self::Int(Sign::None, val)
    }

    pub fn neg_int(val: i128) -> Self {
        Self::Int(Sign::Minus, val)
    }
}

#[derive(Debug)]
pub enum Operand<'a> {
    Var(Ident<'a>),
    Const(Const<'a>),
}

pub trait IntoOperand<'a> {
    fn into_operand(self) -> Operand<'a>;
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Copy)]
pub enum Cmp {
    /// Returns 1 if first value is less than second, signed comparison
    Slt,
    /// Returns 1 if first value is less than or equal to second, respecting signedness
    Sle,
    /// Returns 1 if first value is greater than second, respecting signedness
    Sgt,
    /// Returns 1 if first value is greater than or equal to second, respecting signedness
    Sge,
    /// Returns 1 if values are equal
    Eq,
    /// Returns 1 if values are not equal
    Ne,
    /// Returns 1 if both operands are not NaN (ordered comparison)
    O,
    /// Returns 1 if at least one operand is NaN (unordered comparison)
    Uo,
    /// Returns 1 if first value is less than second, unsigned comparison
    Ult,
    /// Returns 1 if first value is less than or equal to second, unsigned comparison
    Ule,
    /// Returns 1 if first value is greater than second, unsigned comparison
    Ugt,
    /// Returns 1 if first value is greater than or equal to second, unsigned comparison
    Uge,
}

#[derive(Debug)]
pub enum Instruction<'a> {
    /// Adds values of two temporaries together
    Add(Operand<'a>, Operand<'a>),
    /// Subtracts the second value from the first one
    Sub(Operand<'a>, Operand<'a>),
    /// Multiplies values of two temporaries
    Mul(Operand<'a>, Operand<'a>),
    /// Divides the first value by the second one
    Div(Operand<'a>, Operand<'a>),
    /// Returns a remainder from division
    Rem(Operand<'a>, Operand<'a>),
    /// Performs a comparion between values
    Cmp(AbiTy<'a>, Cmp, Operand<'a>, Operand<'a>),
    /// Performs a bitwise AND on values
    And(Operand<'a>, Operand<'a>),
    /// Performs a bitwise OR on values
    Or(Operand<'a>, Operand<'a>),
    /// Performs a bitwise XOR on operands
    Xor(Operand<'a>, Operand<'a>),
    /// Negates a value
    Neg(Operand<'a>),
    /// Copies an operand
    Copy(Operand<'a>),
    /// Calls a function
    Call(String, Vec<(AbiTy<'a>, Operand<'a>)>, Option<u64>),
    /// Allocates a 4-byte aligned area on the stack
    Alloc4(u32),
    /// Allocates a 8-byte aligned area on the stack
    Alloc8(u64),
    /// Allocates a 16-byte aligned area on the stack
    Alloc16(u128),
    /// Stores a value into memory pointed to by destination.
    /// `(type, destination, value)`
    ///
    /// For sub-word types, signed/unsigned variants (`SignedByte`, `UnsignedByte`,
    /// `SignedHalfword`, `UnsignedHalfword`) are accepted and map to `storeb`/`storeh`,
    /// since stores only truncate and don't distinguish signedness.
    ///
    /// See the [QBE IL reference](https://c9x.me/compile/doc/il.html#Memory).
    Store(AbiTy<'a>, Operand<'a>, Operand<'a>),
    /// Loads a value from memory pointed to by source.
    /// `(type, source)`
    ///
    /// # Panics
    ///
    /// Panics if called with [`AbiTy<'a>::Byte`] or [`Type::Halfword`], because QBE requires
    /// explicit sign/zero extension for sub-word loads. Use [`AbiTy<'a>::SignedByte`] /
    /// [`AbiTy<'a>::UnsignedByte`] or [`Type::SignedHalfword`] / [`Type::UnsignedHalfword`]
    /// instead.
    ///
    /// See the [QBE IL reference](https://c9x.me/compile/doc/il.html#Memory).
    Load(AbiTy<'a>, Operand<'a>),
    /// `(source, destination, n)`
    ///
    /// Copy `n` bytes from the source address to the destination address.
    ///
    /// n must be a constant value.
    ///
    /// ## Minimum supported QBE version
    /// `1.1`
    Blit(Operand<'a>, Operand<'a>, u64),

    /// Debug file.
    DbgFile(String),
    /// Debug line.
    ///
    /// Takes line number and an optional column.
    DbgLoc(u64, Option<u64>),

    /// Performs unsigned division of the first value by the second one
    Udiv(Operand<'a>, Operand<'a>),
    /// Returns the remainder from unsigned division
    Urem(Operand<'a>, Operand<'a>),

    /// Shift arithmetic right (preserves sign)
    Sar(Operand<'a>, Operand<'a>),
    /// Shift logical right (fills with zeros)
    Shr(Operand<'a>, Operand<'a>),
    /// Shift left (fills with zeros)
    Shl(Operand<'a>, Operand<'a>),

    /// Cast between integer and floating point of the same width
    Cast(Operand<'a>),

    /// Sign-extends a word to a long
    Extsw(Operand<'a>),
    /// Zero-extends a word to a long
    Extuw(Operand<'a>),
    /// Sign-extends a halfword to a word or long
    Extsh(Operand<'a>),
    /// Zero-extends a halfword to a word or long
    Extuh(Operand<'a>),
    /// Sign-extends a byte to a word or long
    Extsb(Operand<'a>),
    /// Zero-extends a byte to a word or long
    Extub(Operand<'a>),
    /// Extends a single-precision float to double-precision
    Exts(Operand<'a>),
    /// Truncates a double-precision float to single-precision
    Truncd(Operand<'a>),

    /// Converts a single-precision float to a signed integer
    Stosi(Operand<'a>),
    /// Converts a single-precision float to an unsigned integer
    Stoui(Operand<'a>),
    /// Converts a double-precision float to a signed integer
    Dtosi(Operand<'a>),
    /// Converts a double-precision float to an unsigned integer
    Dtoui(Operand<'a>),
    /// Converts a signed word to a float
    Swtof(Operand<'a>),
    /// Converts an unsigned word to a float
    Uwtof(Operand<'a>),
    /// Converts a signed long to a float
    Sltof(Operand<'a>),
    /// Converts an unsigned long to a float
    Ultof(Operand<'a>),

    /// Initializes a variable argument list
    Vastart(Operand<'a>),
    /// Fetches the next argument from a variable argument list
    Vaarg(AbiTy<'a>, Operand<'a>),
    // // Phi instruction
    // /// Selects value based on the control flow path into a block.
    // Phi(Vec<(String, Operand<'a>)>),
}

/// An IR statement
#[derive(Debug)]
pub enum Statement<'a> {
    Assign(Operand<'a>, BaseTy, Instruction<'a>),
    Volatile(Instruction<'a>),
}

#[derive(Debug)]
pub enum Jump<'a> {
    /// Unconditionally jumps to a block
    Jmp(Ident<'a>),
    /// Jumps to first block if a value is nonzero or to the second one otherwise
    Jnz(Operand<'a>, Ident<'a>, Ident<'a>),
    /// Return from a function
    Ret(Operand<'a>), // TODO: could be an option
    /// Halt the program
    Hlt,
}

#[derive(Debug)]
pub struct Block<'a> {
    pub ident: Ident<'a>,
    pub phi_instructions: Vec<Statement<'a>>,
    pub instructions: Vec<Statement<'a>>,
    pub jump: Jump<'a>,
}

#[derive(Debug)]
pub struct Function<'a> {
    pub linkage: Option<Linkage>,
    pub ident: Ident<'a>,
    pub return_ty: Option<AbiTy<'a>>,
    pub params: Vec<Param<'a>>,
    pub blocks: Vec<Block<'a>>,
}

#[derive(Debug)]
pub enum DataItem<'a> {
    Ident(Ident<'a>, Option<Offset>),
    String(Cow<'a, str>),
    Const(Const<'a>),
}

#[derive(Debug)]
pub enum DataValue<'a> {
    Data(Vec<(ExtTy, DataItem<'a>)>),
    Zeroed(Size),
}

#[derive(Debug)]
pub struct Data<'a> {
    pub linkage: Option<Linkage>,
    pub ident: Ident<'a>,
    pub align: Option<Alignment>,
    pub value: DataValue<'a>,
}

#[derive(Debug)]
pub enum Definition<'a> {
    Ty(TyDef<'a>),
    Data(Data<'a>),
    Fn(Function<'a>),
}

#[derive(Debug, Default)]
pub struct Module<'a> {
    pub defs: Vec<Definition<'a>>,
}
