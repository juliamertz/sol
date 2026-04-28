use std::fmt::{Display, Formatter, Result, Write};

use crate::traits::CollectVec;

use super::*;

fn join_fmt(items: impl IntoIterator<Item = impl ToString>, sep: &str) -> String {
    items
        .into_iter()
        .map(|item| item.to_string())
        .collect_vec()
        .join(sep)
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let (sigil, name) = match self {
            Ident::Ty(ty) => (':', ty),
            Ident::Global(global) => ('$', global),
            Ident::Temp(temp) => ('%', temp),
            Ident::Block(block) => ('@', block),
        };
        f.write_char(sigil)?;
        f.write_str(name)
    }
}

impl Display for BaseTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char(match self {
            BaseTy::Word => 'w',
            BaseTy::Long => 'l',
            BaseTy::Single => 's',
            BaseTy::Double => 'd',
        })
    }
}

impl Display for ExtTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char(match self {
            ExtTy::Base(base_ty) => return base_ty.fmt(f),
            ExtTy::Byte => 'b',
            ExtTy::HalfWord => 'h',
        })
    }
}

impl Display for SubWordTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(match self {
            SubWordTy::SignedByte => "sb",
            SubWordTy::UnsignedByte => "ub",
            SubWordTy::SingedHalfWord => "sh",
            SubWordTy::UnsingedHalfWord => "uh",
        })
    }
}

impl Display for AbiTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            AbiTy::Base(base_ty) => base_ty.fmt(f),
            AbiTy::SubWord(sub_word_ty) => sub_word_ty.fmt(f),
            AbiTy::Aggregate(ty_def) => ty_def.ident().fmt(f),
        }
    }
}

impl Display for SubTyKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            SubTyKind::Extended(ext_ty) => ext_ty.fmt(f),
            SubTyKind::Aggregate(ty_def) => f.write_str(ty_def.ident().as_str()),
        }
    }
}

impl Display for SubTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.kind.fmt(f)?;
        if let Some(alignment) = self.align {
            f.write_char(' ')?;
            f.write_str(&alignment.to_string())?;
        }
        Ok(())
    }
}

impl Display for TyDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "type {} = ", self.ident())?;
        write!(f, "align {}", self.align())?;

        f.write_char('{')?;
        match self {
            TyDef::Regular { items: sub_tys, .. } => {
                for (sub_ty, _repeat) in sub_tys {
                    sub_ty.fmt(f)?;
                    f.write_char(',')?;
                }
            }
            TyDef::Union { variants, .. } => {
                f.write_char('{')?;
                for items in variants {
                    for (sub_ty, _repeat) in items {
                        sub_ty.fmt(f)?;
                        f.write_char(',')?;
                    }
                }
                f.write_char('}')?;
            }
            TyDef::Opaque { .. } => unimplemented!(),
        }
        f.write_char('}')
    }
}

impl Display for Linkage {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(match self {
            Linkage::Export => "export",
            Linkage::Thread => "thread",
        })
    }
}

impl Display for RegularParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.fmt(f)?;
        f.write_char(' ')?;
        self.1.fmt(f)
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Param::Regular(param) => param.fmt(f),
            Param::Env(_) => unimplemented!(),
            Param::VariadicMarker => f.write_str("..."),
        }
    }
}

impl Display for Sign {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Sign::Minus => f.write_char('-'),
            Sign::None => Ok(()),
        }
    }
}

impl Display for Precision {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char(match self {
            Precision::Single => 's',
            Precision::Double => 'd',
        })?;
        f.write_char('_')
    }
}

impl Display for Const {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Const::Int(sign, val) => write!(f, "{sign}{val}"),
            Const::Float(precision, val) => write!(f, "{precision}{val}"),
            Const::Ident(ident) => ident.fmt(f),
        }
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Operand::Var(name) => name.fmt(f),
            Operand::Const(constant) => constant.fmt(f),
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Add(lhs, rhs) => write!(f, "add {lhs}, {rhs}"),
            Self::Sub(lhs, rhs) => write!(f, "sub {lhs}, {rhs}"),
            Self::Mul(lhs, rhs) => write!(f, "mul {lhs}, {rhs}"),
            Self::Div(lhs, rhs) => write!(f, "div {lhs}, {rhs}"),
            Self::Rem(lhs, rhs) => write!(f, "rem {lhs}, {rhs}"),
            Self::Cmp(ty, cmp, lhs, rhs) => {
                // TODO:!
                // assert!(
                //     !matches!(ty, Type::Aggregate(_)),
                //     "cannot compare aggregate types"
                // );

                write!(
                    f,
                    "c{}{} {}, {}",
                    match cmp {
                        Cmp::Slt => "slt",
                        Cmp::Sle => "sle",
                        Cmp::Sgt => "sgt",
                        Cmp::Sge => "sge",
                        Cmp::Eq => "eq",
                        Cmp::Ne => "ne",
                        Cmp::O => "o",
                        Cmp::Uo => "uo",
                        Cmp::Ult => "ult",
                        Cmp::Ule => "ule",
                        Cmp::Ugt => "ugt",
                        Cmp::Uge => "uge",
                    },
                    ty,
                    lhs,
                    rhs,
                )
            }
            Self::And(lhs, rhs) => write!(f, "and {lhs}, {rhs}"),
            Self::Or(lhs, rhs) => write!(f, "or {lhs}, {rhs}"),
            Self::Xor(lhs, rhs) => write!(f, "xor {lhs}, {rhs}"),
            Self::Neg(val) => write!(f, "neg {val}"),
            Self::Copy(val) => write!(f, "copy {val}"),
            // Self::Ret(val) => match val {
            //     Some(val) => write!(f, "ret {val}"),
            //     None => write!(f, "ret"),
            // },
            Self::DbgFile(val) => write!(f, r#"dbgfile "{val}""#),
            Self::DbgLoc(lineno, column) => match column {
                Some(val) => write!(f, "dbgloc {lineno}, {val}"),
                None => write!(f, "dbgloc {lineno}"),
            },
            // Self::Jnz(val, if_nonzero, if_zero) => {
            //     write!(f, "jnz {val}, @{if_nonzero}, @{if_zero}")
            // }
            // Self::Jmp(label) => write!(f, "jmp @{label}"),
            Self::Call(name, args, opt_variadic_i) => {
                let mut args_fmt = args
                    .iter()
                    .map(|(ty, temp)| format!("{ty} {temp}"))
                    .collect_vec();
                if let Some(i) = *opt_variadic_i {
                    args_fmt.insert(i as usize, "...".to_string());
                }

                write!(f, "call ${}({})", name, args_fmt.join(", "),)
            }
            Self::Alloc4(size) => write!(f, "alloc4 {size}"),
            Self::Alloc8(size) => write!(f, "alloc8 {size}"),
            Self::Alloc16(size) => write!(f, "alloc16 {size}"),
            Self::Store(ty, dest, value) => {
                let suffix = match ty {
                    // TODO:!
                    // Type::SignedByte | Type::UnsignedByte => "b".to_string(),
                    // Type::SignedHalfword | Type::UnsignedHalfword => "h".to_string(),
                    // Type::Aggregate(_) => panic!("cannot store to an aggregate type"),
                    _ => ty.to_string(),
                };
                write!(f, "store{suffix} {value}, {dest}")
            }
            Self::Load(ty, src) => match ty {
                // TODO:!
                // Type::Byte | Type::Halfword => panic!(
                //     "ambiguous sub-word load: use SignedByte/UnsignedByte or SignedHalfword/UnsignedHalfword"
                // ),
                // Type::Aggregate(_) => panic!("cannot load aggregate type"),
                _ => write!(f, "load{ty} {src}"),
            },
            Self::Blit(src, dst, n) => write!(f, "blit {src}, {dst}, {n}"),
            Self::Udiv(lhs, rhs) => write!(f, "udiv {lhs}, {rhs}"),
            Self::Urem(lhs, rhs) => write!(f, "urem {lhs}, {rhs}"),
            Self::Sar(lhs, rhs) => write!(f, "sar {lhs}, {rhs}"),
            Self::Shr(lhs, rhs) => write!(f, "shr {lhs}, {rhs}"),
            Self::Shl(lhs, rhs) => write!(f, "shl {lhs}, {rhs}"),
            Self::Cast(val) => write!(f, "cast {val}"),
            Self::Extsw(val) => write!(f, "extsw {val}"),
            Self::Extuw(val) => write!(f, "extuw {val}"),
            Self::Extsh(val) => write!(f, "extsh {val}"),
            Self::Extuh(val) => write!(f, "extuh {val}"),
            Self::Extsb(val) => write!(f, "extsb {val}"),
            Self::Extub(val) => write!(f, "extub {val}"),
            Self::Exts(val) => write!(f, "exts {val}"),
            Self::Truncd(val) => write!(f, "truncd {val}"),
            Self::Stosi(val) => write!(f, "stosi {val}"),
            Self::Stoui(val) => write!(f, "stoui {val}"),
            Self::Dtosi(val) => write!(f, "dtosi {val}"),
            Self::Dtoui(val) => write!(f, "dtoui {val}"),
            Self::Swtof(val) => write!(f, "swtof {val}"),
            Self::Uwtof(val) => write!(f, "uwtof {val}"),
            Self::Sltof(val) => write!(f, "sltof {val}"),
            Self::Ultof(val) => write!(f, "ultof {val}"),
            Self::Vastart(val) => write!(f, "vastart {val}"),
            Self::Vaarg(ty, val) => write!(f, "vaarg{ty} {val}"),
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Assign(temp, ty, instr) => {
                assert!(
                    matches!(temp, Operand::Var(_)),
                    "assignment target must be a temporary, got {temp:?}"
                );
                write!(f, "{temp} ={ty} {instr}")
            }
            Self::Volatile(instr) => write!(f, "{instr}"),
        }
    }
}

impl Display for Jump {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Jump::Jmp(ident) => write!(f, "jmp {ident}"),
            Jump::Jnz(operand, lhs, rhs) => {
                write!(f, "jnz {operand}, {lhs}, {rhs}")
            }
            Jump::Ret(operand) => write!(f, "ret {operand}"),
            Jump::Hlt => write!(f, "hlt"),
        }
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ident.fmt(f)?;
        f.write_char('\n')?;
        for instruction in self.instructions.iter() {
            writeln!(f, "\t{instruction}")?;
        }
        writeln!(f, "\t{}", self.jump)?;
        Ok(())
    }
}

impl Display for DataItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DataItem::Ident(_ident, _) => todo!(),
            DataItem::String(val) => write!(f, "\"{val}\""),
            DataItem::Const(val) => val.fmt(f),
        }
    }
}

impl Display for DataValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DataValue::Data(items) => f.write_str(
                &items
                    .iter()
                    .map(|(ty, item)| format!("{ty} {item}"))
                    .collect_vec()
                    .join(", "),
            ),
            DataValue::Zeroed(_) => todo!(),
        }
    }
}

impl Display for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(
            f,
            "data {ident} = {{{val}}}",
            ident = self.ident,
            val = self.value
        )
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if let Some(linkage) = self.linkage.as_ref() {
            linkage.fmt(f)?;
            f.write_char(' ')?;
        }
        f.write_str("function ")?;
        if let Some(return_ty) = self.return_ty.as_ref() {
            return_ty.fmt(f)?;
            f.write_char(' ')?;
        }
        self.ident.fmt(f)?;
        f.write_char('(')?;
        let params = join_fmt(&self.params, ", ");
        f.write_str(&params)?;
        f.write_char(')')?;
        f.write_char(' ')?;
        f.write_char('{')?;
        f.write_char('\n')?;
        for block in self.blocks.iter() {
            block.fmt(f)?;
            f.write_char('\n')?;
        }
        f.write_char('}')?;
        Ok(())
    }
}

impl Display for Definition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Definition::Ty(ty) => ty.fmt(f),
            Definition::Data(data) => data.fmt(f),
            Definition::Fn(function) => function.fmt(f),
        }
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for definition in self.defs.iter() {
            definition.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}
