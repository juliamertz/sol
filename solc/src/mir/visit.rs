use super::*;

pub trait Visitor<T> {
    fn visit_module(&mut self, module: &Module) -> T;
    fn visit_definition(&mut self, def: &Definition) -> T;
    fn visit_ty_def(&mut self, ty_def: &TyDef) -> T;
    fn visit_data(&mut self, data: &Data) -> T;
    fn visit_func(&mut self, func: &Fn) -> T;
    fn visit_block(&mut self, block: &Block) -> T;
    fn visit_instruction(&mut self, instruction: &Instruction) -> T;
    fn visit_terminator(&mut self, terminator: &Terminator) -> T;
    fn visit_operand(&mut self, operand: &Operand) -> T;
    fn visit_constant(&mut self, constant: &Constant) -> T;
    fn visit_temp_id(&mut self, temp_id: &TempId) -> T;
    fn visit_data_id(&mut self, data_id: &DataId) -> T;
    fn visit_block_id(&mut self, block_id: &BlockId) -> T;
}
