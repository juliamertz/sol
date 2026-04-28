use crate::hir;
use crate::mir::builder::{Builder, BuilderError};
use crate::mir::{Data, Definition, Fn, Indirection, MirTy, Module, Operand, Terminator, TyDef};
use crate::traits::{AsStr, CollectVec};
use crate::type_checker::{TypeEnv, TypeId};

fn lower_func(
    ident: &hir::Ident<'_>,
    return_ty_id: TypeId,
    params: &[(hir::Ident<'_>, TypeId)],
    body: &hir::Block<'_>,
    env: &TypeEnv,
) -> Result<(Fn, Vec<Data>), BuilderError> {
    let mut builder = Builder::new(env);
    let entry = builder.new_block();

    let mut lowered_params = vec![];
    let mut return_ty = MirTy::new(return_ty_id);

    if env.types.get(&return_ty_id).must_allocate() {
        return_ty.set_indirection(Indirection::Ptr);
        let temp_id = builder.new_temp(return_ty);
        lowered_params.push((temp_id, return_ty));
        builder.set_return_destination(temp_id);
    }

    for (ident, ty_id) in params {
        let param_ty = MirTy::new(*ty_id);
        let temp_id = builder.new_temp(param_ty);
        let operand = Operand::Temporary(temp_id);
        builder.define_local(ident.def_id, operand);
        lowered_params.push((temp_id, param_ty))
    }

    let (val, exit_block) = builder.lower_block(body, entry)?;

    builder
        .get_block_mut(&exit_block)
        .terminate(Terminator::Return(val))?;

    let name = env
        .def_names
        .get(&ident.def_id)
        .map(|s| s.as_ref())
        .unwrap_or_else(|| ident.as_str());

    builder.build(name, return_ty, lowered_params.into_iter())
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
            hir::FnKind::Extern { .. } => None, // TODO:
        },
        hir::Item::TyDef(ty_def) => {
            let hir::TyDef::Struct {
                ident,
                items,
                fields,
                ..
            } = ty_def;
            let mut defs = vec![Definition::Ty(TyDef::Struct {
                name: ident.as_str().to_string(),
                fields: fields
                    .iter()
                    .map(|(field_id, ty_id)| (*field_id, MirTy::new(*ty_id)))
                    .collect(),
            })];

            for (_id, item) in items.iter() {
                let hir::AssocItem::Fn(func) = item;
                if let hir::FnKind::Local { params, body } = &func.kind {
                    let (mir_fn, data) =
                        lower_func(&func.ident, func.return_ty, params, body, env)?;
                    defs.push(Definition::Fn(mir_fn));
                    defs.extend(data.into_iter().map(Definition::Data));
                }
            }

            Some(defs)
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
        .collect_vec();

    Ok(Module { defs })
}
