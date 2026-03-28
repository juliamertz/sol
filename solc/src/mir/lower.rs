use crate::ext::AsStr;
use crate::hir;
use crate::mir::builder::{Builder, BuilderError};
use crate::mir::{Data, DataValue, Definition, Fn, Module, Operand, Terminator};
use crate::type_checker::{TypeEnv, TypeId};

fn lower_func(
    ident: &hir::Ident<'_>,
    return_ty: TypeId,
    params: &[(hir::Ident<'_>, TypeId)],
    body: &hir::Block<'_>,
    env: &TypeEnv,
) -> Result<(Fn, Vec<Data>), BuilderError> {
    let mut builder = Builder::new(env);
    let entry = builder.new_block();

    for (ident, ty_id) in params {
        let operand = Operand::Temporary(builder.new_temp(*ty_id));
        dbg!(&ident);
        builder.define_local(ident.def_id, operand);
    }

    let (val, exit_block) = builder.lower_hir_block(body, entry)?;

    builder
        .get_block_mut(&exit_block)
        .terminate(Terminator::Return(val))?;

    let param_tys = params.iter().map(|(_, ty)| *ty);
    builder.build(ident.as_str(), return_ty, param_tys)
}

pub fn lower_item(
    item: &hir::Item<'_>,
    env: &TypeEnv,
) -> Result<Option<Vec<Definition>>, BuilderError> {
    let defs = match item {
        hir::Item::Use(_) => None,
        hir::Item::Fn(func) => match &func.kind {
            hir::FnKind::Local { params, body } => {
                let (func, data) = lower_func(&func.ident, func.return_ty, params, body, env)?;
                let mut defs = vec![Definition::Fn(func)];
                defs.extend(data.into_iter().map(Definition::Data));
                Some(defs)
            }
            hir::FnKind::Extern { params: _ } => None, // TODO:
        },
        hir::Item::StructDef(struct_def) => {
            let ty = env.type_by_id(&struct_def.ident.ty).unwrap(); // TODO: kind of weird that we resolve this type by ident
            let def = Definition::Ty(ty.clone());
            Some(vec![def])
        }
    };

    Ok(defs)
}

pub fn lower_module(module: &hir::Module<'_>, env: &TypeEnv) -> Result<Module, BuilderError> {
    let defs = module
        .items
        .iter()
        .map(|item| lower_item(item, env))
        .collect::<Result<Vec<_>, BuilderError>>()?
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>();

    Ok(Module { defs })
}
