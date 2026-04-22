use std::rc::Rc;

pub mod build;
pub mod fmt;
pub mod lower;

pub type Alignment = u64;
pub type Size = u64;
pub type Offset = u64;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Ident {
    Ty(String),
    Global(String),
    Temp(String),
    Block(String),
}

impl Ident {
    pub fn ty(value: impl ToString) -> Self {
        Self::Ty(value.to_string())
    }

    pub fn global(value: impl ToString) -> Self {
        Self::Global(value.to_string())
    }

    pub fn temp(value: impl ToString) -> Self {
        Self::Temp(value.to_string())
    }

    pub fn block(value: impl ToString) -> Self {
        Self::Block(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Ident::Ty(inner) => inner,
            Ident::Global(inner) => inner,
            Ident::Temp(inner) => inner,
            Ident::Block(inner) => inner,
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
    pub fn size(&self) -> Size {
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
    pub fn size(&self) -> Size {
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
    pub fn size(&self) -> Size {
        match self {
            Self::SignedByte | Self::UnsignedByte => 1,
            Self::SingedHalfWord | Self::UnsingedHalfWord => 2,
        }
    }
}

#[derive(Debug)]
pub enum AbiTy {
    Base(BaseTy),
    SubWord(SubWordTy),
    Aggregate(Rc<TyDef>),
}

impl AbiTy {
    /// size in bytes
    pub fn size(&self) -> Size {
        match self {
            AbiTy::Base(base_ty) => base_ty.size(),
            AbiTy::SubWord(sub_word_ty) => sub_word_ty.size(),
            AbiTy::Aggregate(ty_def) => ty_def.size(),
        }
    }

    pub fn align(&self) -> Alignment {
        self.size()
    }

    pub fn as_base(&self) -> BaseTy {
        match self {
            AbiTy::Base(base_ty) => *base_ty,
            AbiTy::SubWord(_) => BaseTy::Word,
            AbiTy::Aggregate(_) => BaseTy::Long,
        }
    }

    pub fn as_sub_ty(&self) -> SubTy {
        let kind = match self {
            AbiTy::Base(base_ty) => SubTyKind::Extended(ExtTy::Base(*base_ty)),
            AbiTy::SubWord(_) => todo!(), // TODO: this can fit into something else.
            AbiTy::Aggregate(ty_def) => SubTyKind::Aggregate(ty_def.clone()),
        };
        SubTy { kind, align: None }
    }

    pub fn as_aggregate(&self) -> Option<&TyDef> {
        match self {
            Self::Aggregate(ty_def) => Some(ty_def),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum SubTyKind {
    Extended(ExtTy),
    Aggregate(Rc<TyDef>),
}

#[derive(Debug)]
pub struct SubTy {
    pub kind: SubTyKind,
    pub align: Option<Alignment>,
}

impl SubTy {
    pub fn size(&self) -> Size {
        match &self.kind {
            SubTyKind::Extended(ext_ty) => ext_ty.size(),
            SubTyKind::Aggregate(ty_def) => ty_def.size(),
        }
    }

    pub fn align(&self) -> Alignment {
        self.align.unwrap_or(self.size())
    }
}

#[derive(Debug)]
pub enum TyDef {
    Regular {
        ident: Ident,
        align: Option<Alignment>,
        items: Vec<(SubTy, u64)>,
    },
    Union {
        ident: Ident,
        align: Option<Alignment>,
        variants: Vec<Vec<(SubTy, u64)>>,
    },
    Opaque {
        ident: Ident,
        align: Alignment,
        size: Size,
    },
}

impl TyDef {
    fn ident(&self) -> &Ident {
        match self {
            TyDef::Regular { ident, .. } => ident,
            TyDef::Union { ident, .. } => ident,
            TyDef::Opaque { ident, .. } => ident,
        }
    }

    pub fn size(&self) -> Size {
        let size_of_items = |this: &TyDef, items: &[(SubTy, u64)]| {
            let mut offset = 0;

            for (item, repeat) in items.iter() {
                let align = item.align();
                let size = *repeat * item.size();
                let padding = (align - (offset % align)) % align;
                offset += padding + size;
            }

            let align = this.align();
            let padding = (align - (offset % align)) % align;

            offset + padding
        };

        match self {
            TyDef::Regular { items, .. } => size_of_items(self, items),
            TyDef::Union { variants, .. } => variants
                .iter()
                .map(|variant| size_of_items(self, &variant))
                .max()
                .unwrap_or(0),
            TyDef::Opaque { size, .. } => *size,
        }
    }

    pub fn align(&self) -> Alignment {
        let alignment_of_items = |items: &[(SubTy, _)]| {
            items
                .iter()
                .map(|(sub_ty, _)| sub_ty.align())
                .max()
                .unwrap_or(1)
        };

        match self {
            TyDef::Regular { align, items, .. } => {
                align.unwrap_or_else(|| alignment_of_items(items))
            }
            TyDef::Union {
                align, variants, ..
            } => align.unwrap_or_else(|| {
                variants
                    .iter()
                    .map(|items| alignment_of_items(items))
                    .max()
                    .unwrap_or(1)
            }),
            TyDef::Opaque { align, .. } => *align,
        }
    }

    /// get byte offset for accessing field
    pub fn offset_for(&self, id: u32) -> Offset {
        let TyDef::Regular { items, .. } = self else {
            unimplemented!()
        };

        items
            .iter()
            .take(id as usize)
            .map(|(sub_ty, repeat)| sub_ty.size() * repeat)
            .sum()
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
pub struct RegularParam(pub AbiTy, pub Operand);

#[derive(Debug)]
pub enum Param {
    Regular(RegularParam),
    Env(Ident),
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
pub enum Const {
    Int(Sign, i128),
    Float(Precision, f64),
    Ident(Ident),
}

impl Const {
    pub fn int(val: i128) -> Self {
        Self::Int(Sign::None, val)
    }

    pub fn neg_int(val: i128) -> Self {
        Self::Int(Sign::Minus, val)
    }
}

#[derive(Debug)]
pub enum Operand {
    Var(Ident),
    Const(Const),
}

pub trait IntoOperand {
    fn into_operand(self) -> Operand;
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
pub enum Instruction {
    /// Adds values of two temporaries together
    Add(Operand, Operand),
    /// Subtracts the second value from the first one
    Sub(Operand, Operand),
    /// Multiplies values of two temporaries
    Mul(Operand, Operand),
    /// Divides the first value by the second one
    Div(Operand, Operand),
    /// Returns a remainder from division
    Rem(Operand, Operand),
    /// Performs a comparion between values
    Cmp(AbiTy, Cmp, Operand, Operand),
    /// Performs a bitwise AND on values
    And(Operand, Operand),
    /// Performs a bitwise OR on values
    Or(Operand, Operand),
    /// Performs a bitwise XOR on operands
    Xor(Operand, Operand),
    /// Negates a value
    Neg(Operand),
    /// Copies an operand
    Copy(Operand),
    /// Calls a function
    Call(String, Vec<(AbiTy, Operand)>, Option<u64>),
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
    Store(AbiTy, Operand, Operand),
    /// Loads a value from memory pointed to by source.
    /// `(type, source)`
    ///
    /// # Panics
    ///
    /// Panics if called with [`AbiTy::Byte`] or [`Type::Halfword`], because QBE requires
    /// explicit sign/zero extension for sub-word loads. Use [`AbiTy::SignedByte`] /
    /// [`AbiTy::UnsignedByte`] or [`Type::SignedHalfword`] / [`Type::UnsignedHalfword`]
    /// instead.
    ///
    /// See the [QBE IL reference](https://c9x.me/compile/doc/il.html#Memory).
    Load(AbiTy, Operand),
    /// `(source, destination, n)`
    ///
    /// Copy `n` bytes from the source address to the destination address.
    ///
    /// n must be a constant value.
    ///
    /// ## Minimum supported QBE version
    /// `1.1`
    Blit(Operand, Operand, u64),

    /// Debug file.
    DbgFile(String),
    /// Debug line.
    ///
    /// Takes line number and an optional column.
    DbgLoc(u64, Option<u64>),

    /// Performs unsigned division of the first value by the second one
    Udiv(Operand, Operand),
    /// Returns the remainder from unsigned division
    Urem(Operand, Operand),

    /// Shift arithmetic right (preserves sign)
    Sar(Operand, Operand),
    /// Shift logical right (fills with zeros)
    Shr(Operand, Operand),
    /// Shift left (fills with zeros)
    Shl(Operand, Operand),

    /// Cast between integer and floating point of the same width
    Cast(Operand),

    /// Sign-extends a word to a long
    Extsw(Operand),
    /// Zero-extends a word to a long
    Extuw(Operand),
    /// Sign-extends a halfword to a word or long
    Extsh(Operand),
    /// Zero-extends a halfword to a word or long
    Extuh(Operand),
    /// Sign-extends a byte to a word or long
    Extsb(Operand),
    /// Zero-extends a byte to a word or long
    Extub(Operand),
    /// Extends a single-precision float to double-precision
    Exts(Operand),
    /// Truncates a double-precision float to single-precision
    Truncd(Operand),

    /// Converts a single-precision float to a signed integer
    Stosi(Operand),
    /// Converts a single-precision float to an unsigned integer
    Stoui(Operand),
    /// Converts a double-precision float to a signed integer
    Dtosi(Operand),
    /// Converts a double-precision float to an unsigned integer
    Dtoui(Operand),
    /// Converts a signed word to a float
    Swtof(Operand),
    /// Converts an unsigned word to a float
    Uwtof(Operand),
    /// Converts a signed long to a float
    Sltof(Operand),
    /// Converts an unsigned long to a float
    Ultof(Operand),

    /// Initializes a variable argument list
    Vastart(Operand),
    /// Fetches the next argument from a variable argument list
    Vaarg(AbiTy, Operand),
    // // Phi instruction
    // /// Selects value based on the control flow path into a block.
    // Phi(Vec<(String, Operand)>),
}

/// An IR statement
#[derive(Debug)]
pub enum Statement {
    Assign(Operand, BaseTy, Instruction),
    Volatile(Instruction),
}

#[derive(Debug)]
pub enum Jump {
    /// Unconditionally jumps to a block
    Jmp(Ident),
    /// Jumps to first block if a value is nonzero or to the second one otherwise
    Jnz(Operand, Ident, Ident),
    /// Return from a function
    Ret(Operand), // TODO: could be an option
    /// Halt the program
    Hlt,
}

#[derive(Debug)]
pub struct Block {
    pub ident: Ident,
    pub phi_instructions: Vec<Statement>,
    pub instructions: Vec<Statement>,
    pub jump: Jump,
}

#[derive(Debug)]
pub struct Function {
    pub linkage: Option<Linkage>,
    pub ident: Ident,
    pub return_ty: Option<AbiTy>,
    pub params: Vec<Param>,
    pub blocks: Vec<Block>,
}

#[derive(Debug)]
pub enum DataItem {
    Ident(Ident, Option<Offset>),
    String(String),
    Const(Const),
}

#[derive(Debug)]
pub enum DataValue {
    Data(Vec<(ExtTy, DataItem)>),
    Zeroed(Size),
}

#[derive(Debug)]
pub struct Data {
    pub linkage: Option<Linkage>,
    pub ident: Ident,
    pub align: Option<Alignment>,
    pub value: DataValue,
}

#[derive(Debug)]
pub enum Definition {
    Ty(Rc<TyDef>),
    Data(Data),
    Fn(Function),
}

#[derive(Debug, Default)]
pub struct Module {
    pub defs: Vec<Definition>,
}
