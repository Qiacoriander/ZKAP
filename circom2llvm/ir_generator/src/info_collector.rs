use crate::{scope_information::ScopeInformation, template::TemplateInformation};
use program_structure::ast::{Expression, SignalType, Statement, VariableType};

pub fn collect_depended_components<'ctx>(
    stmt: &Statement,
    scope_info: &mut ScopeInformation<'ctx>,
) {
    match stmt {
        // component c
        // 如果这个Statement是声明了一个组件，var2comp记录变量name->"" (因为只是声明，具体会是哪个组件还未定)
        Statement::Declaration { xtype, name, .. } => match xtype {
            VariableType::Component => {
                scope_info.set_component_var(name, &"".to_string());
            }
            _ => (),
        },
        // component c = Add(a,b)
        // 如果这个Statement是赋值了一个组件，var2comp记录变量name->id(组件名)
        // 同时把id加入当前作用域的依赖列表中
        Statement::Substitution { var, rhe, .. } => match rhe {
            Expression::Call { id, .. } => {
                if scope_info.is_component_var(var) {
                    scope_info.set_component_var(var, id);
                }
                scope_info.set_dependence(id);
            }
            _ => (),
        },
        _ => (),
    }
}

pub fn collect_dependences<'ctx>(expr: &Expression, scope_info: &mut ScopeInformation<'ctx>) {
    match expr {
        Expression::Call { id, .. } => {
            scope_info.set_dependence(id);
        }
        _ => (),
    }
}

pub fn init_template_info(stmts: &Vec<&Statement>) -> TemplateInformation {
    let mut template = TemplateInformation {
        inputs: Vec::new(),
        inters: Vec::new(),
        outputs: Vec::new(),
    };
    for stmt in stmts {
        match stmt {
            Statement::InitializationBlock {
                meta: _,
                xtype,
                initializations,
            } => match xtype {
                VariableType::Signal(signal_type, _) => {
                    for init in initializations {
                        match init {
                            Statement::Declaration { name, .. } => {
                                match signal_type {
                                    SignalType::Input => {
                                        template.add_input(name);
                                    }
                                    SignalType::Intermediate => {
                                        template.add_intermediate(name);
                                    }
                                    SignalType::Output => {
                                        template.add_output(name);
                                    }
                                };
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
    }
    template
}
