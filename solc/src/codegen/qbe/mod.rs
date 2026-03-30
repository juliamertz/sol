use std::borrow::Cow;

use crate::mir;

pub mod build;
pub mod fmt;
pub mod lower;

pub type Alignment = usize;
pub type Size = usize;
pub type Offset = usize;

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
}

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

pub enum ExtTy {
    /// base type
    Base(BaseTy),
    /// 8-bit int
    Byte,
    /// 16-bit int
    HalfWord,
}

pub enum SubWordTy {
    SignedByte,
    UnsignedByte,
    SingedHalf,
    UnsingedHalf,
}

pub enum AbiTy<'a> {
    Base(BaseTy),
    SubWord(SubWordTy),
    Ident(&'a str),
}

pub enum SubTyKind<'a> {
    Extended(ExtTy),
    Ident(&'a str),
}

pub struct SubTy<'a> {
    pub kind: SubTyKind<'a>,
    pub align: Option<Alignment>,
}

pub enum Ty<'a> {
    Aggregate {
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

impl Ty<'_> {
    fn ident(&self) -> &Ident<'_> {
        match self {
            Ty::Aggregate { ident, .. } => ident,
            Ty::Union { ident, .. } => ident,
            Ty::Opaque { ident, .. } => ident,
        }
    }

    pub fn align(&self) -> Option<Alignment> {
        match self {
            Ty::Aggregate { align, .. } => *align,
            Ty::Union { align, .. } => *align,
            Ty::Opaque { align, .. } => Some(*align),
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
pub enum Linkage {
    Export,
    Thread,
    // SecName,
    // SecFlags,
}

pub struct RegularParam<'a>(pub AbiTy<'a>, pub Operand<'a>);

pub enum Param<'a> {
    Regular(RegularParam<'a>),
    Env(Ident<'a>),
    VariadicMarker,
}

pub enum Sign {
    Minus,
    None,
}

pub enum Precision {
    Single,
    Double,
}

// CONST :=
//     ['-'] NUMBER  # Decimal integer
//   | 's_' FP       # Single-precision float
//   | 'd_' FP       # Double-precision float
//   | $IDENT        # Global symbol
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

pub enum Operand<'a> {
    Var(Ident<'a>),
    Const(Const<'a>),
}

pub enum InstructionKind<'a> {
    Basic(&'static str, Vec<Operand<'a>>),
    Call(Ident<'a>, Vec<Param<'a>>),
}

pub struct Instruction<'a> {
    pub ident: Ident<'a>,
    pub return_ty: AbiTy<'a>,
    pub kind: InstructionKind<'a>,
}

impl<'a> Instruction<'a> {
    pub fn new(
        kind: &'static str,
        ident: Ident<'a>,
        return_ty: AbiTy<'a>,
        operands: Vec<Operand<'a>>,
    ) -> Self {
        Self {
            ident,
            return_ty,
            kind: InstructionKind::Basic(kind, operands),
        }
    }
}

pub enum Jump<'a> {
    Jmp(Ident<'a>),
    Jnz(Operand<'a>, Ident<'a>, Ident<'a>),
    Ret(Operand<'a>),
    Hlt,
}

pub struct Block<'a> {
    pub ident: Ident<'a>,
    pub phi_instructions: Vec<Instruction<'a>>,
    pub instructions: Vec<Instruction<'a>>,
    pub jump: Jump<'a>,
}

pub struct Function<'a> {
    pub linkage: Option<Linkage>,
    pub ident: Ident<'a>,
    pub return_ty: Option<AbiTy<'a>>,
    pub params: Vec<Param<'a>>,
    pub blocks: Vec<Block<'a>>,
}

pub enum DataItem<'a> {
    Ident(Ident<'a>, Option<Offset>),
    String(Cow<'a, str>),
    Const(Const<'a>),
}

pub enum DataValue<'a> {
    Data(Vec<(ExtTy, DataItem<'a>)>),
    Zeroed(Size),
}

pub struct Data<'a> {
    pub linkage: Option<Linkage>,
    pub ident: Ident<'a>,
    pub align: Option<Alignment>,
    pub value: DataValue<'a>,
}

pub enum Definition<'a> {
    Ty(Ty<'a>),
    Data(Data<'a>),
    Fn(Function<'a>),
}

#[derive(Default)]
pub struct Module<'a> {
    pub defs: Vec<Definition<'a>>,
}

macro_rules! gen_instruction_methods {
    ($($name:ident)*) => {
        paste::paste! {
            impl<'a> Instruction<'a> {
                $(
                    pub const [<$name:upper>]: &'static str = stringify!($name);

                    pub fn $name(ident: Ident<'a>, return_ty: AbiTy<'a>, operands: Vec<Operand<'a>>) -> Self {
                        Self {
                            ident,
                            return_ty,
                            kind: InstructionKind::Basic(stringify!($name), operands),
                        }
                    }
                )*
            }
        }
    };
}

gen_instruction_methods! {
    // arithmetic and bits
    add
    and
    div
    mul
    neg
    or
    rem
    sar
    shl
    shr
    sub
    udiv
    urem
    xor

    // memory
    alloc16
    alloc4
    alloc8
    blit
    loadd
    loadl
    loads
    loadsb
    loadsh
    loadsw
    loadub
    loaduh
    loaduw
    loadw
    storeb
    stored
    storeh
    storel
    stores
    storew

    // comparisons
    ceqd
    ceql
    ceqs
    ceqw
    cged
    cges
    cgtd
    cgts
    cled
    cles
    cltd
    clts
    cned
    cnel
    cnes
    cnew
    cod
    cos
    csgel
    csgew
    csgtl
    csgtw
    cslel
    cslew
    csltl
    csltw
    cugel
    cugew
    cugtl
    cugtw
    culel
    culew
    cultl
    cultw
    cuod
    cuos

    // conversions
    dtosi
    dtoui
    exts
    extsb
    extsh
    extsw
    extub
    extuh
    extuw
    sltof
    ultof
    stosi
    stoui
    swtof
    uwtof
    truncd

    // cast and copy
    cast
    copy

    // call
    call

    // variadic
    vastart
    vaarg

    // phi
    phi
}
