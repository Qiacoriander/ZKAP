use crate::codegen::CodeGen;
use crate::environment::GlobalInformation;
use crate::namer::{name_body_block, name_entry_block};
use crate::scope::Scope;
use crate::scope_information::ScopeInformation;
use crate::statement::{flat_statements, resolve_stmt};
use crate::type_infer::{get_type_of_expr, infer_ty_from_stmt};
use inkwell::types::BasicType;
use program_structure::ast::Statement;

pub struct Function<'ctx> {
    pub scope: Scope<'ctx>,
}

pub fn infer_fn<'ctx>(
    env: &GlobalInformation<'ctx>,
    scope_info: &mut ScopeInformation<'ctx>,
    body: &Statement,
) {
    let mut ret_ty = env.val_ty.as_basic_type_enum();
    let stmts = flat_statements(body);
    for stmt in &stmts {
        infer_ty_from_stmt(env, scope_info, stmt);
    }
    for stmt in &stmts {
        match stmt {
            Statement::Return { meta: _, value } => {
                let ty = get_type_of_expr(env, &scope_info, value);
                match ty {
                    Some(ty) => ret_ty = ty,
                    None => (),
                }
            }
            _ => (),
        }
    }
    scope_info.set_ret_ty(ret_ty);  // 可以确定function的返回值类型
    let mut arg_tys = Vec::new();
    for a in &scope_info.args {
        arg_tys.push(scope_info.get_var_used_ty(a));
    }
    scope_info.set_arg_tys(arg_tys);  // 可以确定function的参数类型
    scope_info.check();
}

impl<'ctx> Function<'ctx> {
    /// 构建函数的LLVM IR
    /// 所有构建好的结果都被最终存储找CodeGen的module字段中
    pub fn build(
        &mut self,
        env: &GlobalInformation<'ctx>,
        codegen: &CodeGen<'ctx>,
        body: &Statement,
    ) {
        let fn_name = self.scope.get_name().clone();
        let ret_ty = self.scope.info.get_ret_ty();
        let fn_ty = ret_ty.fn_type(&self.scope.info.gen_arg_metadata_tys(), false);
        let fn_val = codegen.module.add_function(&fn_name, fn_ty, None);
        self.scope.set_main_fn(fn_val);

        // 创建入口基本块，并设置为当前退出块
        let entry_bb = codegen
            .context
            .append_basic_block(fn_val, &name_entry_block());
        self.scope.set_current_exit_block(codegen, entry_bb);

        // Bind args
        // 绑定函数参数到scope中
        for (idx, arg) in self.scope.info.args.clone().iter().enumerate() {
            let val = fn_val.get_nth_param(idx as u32).unwrap();
            self.scope.set_arg_val(arg, &val);
        }

        // Initial variable
        // 初始化局部变量
        let var_table = self.scope.info.get_var2ty();
        for (name, ty) in &var_table {
            // 参数不用初始化
            if self.scope.info.is_arg(&name) {
                continue;
            }
            let alloca_name = name;
            self.scope.initial_var(codegen, name, alloca_name, ty, true);
        }

        // 创建函数体的基本块，并建立从【入口块】到【函数体块】的跳转
        let body_bb = codegen
            .context
            .append_basic_block(fn_val, &name_body_block());
        codegen.build_block_transferring(entry_bb, body_bb);
        self.scope.set_current_exit_block(codegen, body_bb);

        // 递归处理函数体中的每个Statement
        match body {
            Statement::Block { meta: _, stmts } => {
                for stmt in stmts {
                    if stmt.is_return() {
                        self.scope.build_exit(codegen);
                    }
                    // 递归处理
                    resolve_stmt(env, codegen, &mut self.scope, stmt);
                }
            }
            _ => unreachable!(),
        }
    }
}
