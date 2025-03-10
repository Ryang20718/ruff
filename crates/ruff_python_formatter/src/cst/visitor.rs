use rustpython_parser::ast::Constant;

use crate::cst::{
    Alias, Arg, Arguments, Body, BoolOp, CmpOp, Comprehension, Excepthandler, ExcepthandlerKind,
    Expr, ExprContext, ExprKind, Keyword, MatchCase, Operator, Pattern, PatternKind, SliceIndex,
    SliceIndexKind, Stmt, StmtKind, UnaryOp, Withitem,
};

pub(crate) trait Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a mut Stmt) {
        walk_stmt(self, stmt);
    }
    fn visit_annotation(&mut self, expr: &'a mut Expr) {
        walk_expr(self, expr);
    }
    fn visit_expr(&mut self, expr: &'a mut Expr) {
        walk_expr(self, expr);
    }
    fn visit_constant(&mut self, constant: &'a mut Constant) {
        walk_constant(self, constant);
    }
    fn visit_expr_context(&mut self, expr_context: &'a mut ExprContext) {
        walk_expr_context(self, expr_context);
    }
    fn visit_bool_op(&mut self, bool_op: &'a mut BoolOp) {
        walk_bool_op(self, bool_op);
    }
    fn visit_operator(&mut self, operator: &'a mut Operator) {
        walk_operator(self, operator);
    }
    fn visit_unary_op(&mut self, unary_op: &'a mut UnaryOp) {
        walk_unary_op(self, unary_op);
    }
    fn visit_cmp_op(&mut self, cmp_op: &'a mut CmpOp) {
        walk_cmp_op(self, cmp_op);
    }
    fn visit_comprehension(&mut self, comprehension: &'a mut Comprehension) {
        walk_comprehension(self, comprehension);
    }
    fn visit_excepthandler(&mut self, excepthandler: &'a mut Excepthandler) {
        walk_excepthandler(self, excepthandler);
    }
    fn visit_slice_index(&mut self, slice_index: &'a mut SliceIndex) {
        walk_slice_index(self, slice_index);
    }
    fn visit_format_spec(&mut self, format_spec: &'a mut Expr) {
        walk_expr(self, format_spec);
    }
    fn visit_arguments(&mut self, arguments: &'a mut Arguments) {
        walk_arguments(self, arguments);
    }
    fn visit_arg(&mut self, arg: &'a mut Arg) {
        walk_arg(self, arg);
    }
    fn visit_keyword(&mut self, keyword: &'a mut Keyword) {
        walk_keyword(self, keyword);
    }
    fn visit_alias(&mut self, alias: &'a mut Alias) {
        walk_alias(self, alias);
    }
    fn visit_withitem(&mut self, withitem: &'a mut Withitem) {
        walk_withitem(self, withitem);
    }
    fn visit_match_case(&mut self, match_case: &'a mut MatchCase) {
        walk_match_case(self, match_case);
    }
    fn visit_pattern(&mut self, pattern: &'a mut Pattern) {
        walk_pattern(self, pattern);
    }
    fn visit_body(&mut self, body: &'a mut Body) {
        walk_body(self, body);
    }
}

pub(crate) fn walk_body<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, body: &'a mut Body) {
    for stmt in &mut body.node {
        visitor.visit_stmt(stmt);
    }
}

pub(crate) fn walk_stmt<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, stmt: &'a mut Stmt) {
    match &mut stmt.node {
        StmtKind::FunctionDef {
            args,
            body,
            decorator_list,
            returns,
            ..
        } => {
            visitor.visit_arguments(args);
            for expr in decorator_list {
                visitor.visit_expr(expr);
            }
            for expr in returns {
                visitor.visit_annotation(expr);
            }
            visitor.visit_body(body);
        }
        StmtKind::AsyncFunctionDef {
            args,
            body,
            decorator_list,
            returns,
            ..
        } => {
            visitor.visit_arguments(args);
            for expr in decorator_list {
                visitor.visit_expr(expr);
            }
            for expr in returns {
                visitor.visit_annotation(expr);
            }
            visitor.visit_body(body);
        }
        StmtKind::ClassDef {
            bases,
            keywords,
            body,
            decorator_list,
            ..
        } => {
            for expr in bases {
                visitor.visit_expr(expr);
            }
            for keyword in keywords {
                visitor.visit_keyword(keyword);
            }
            for expr in decorator_list {
                visitor.visit_expr(expr);
            }
            visitor.visit_body(body);
        }
        StmtKind::Return { value } => {
            if let Some(expr) = value {
                visitor.visit_expr(expr);
            }
        }
        StmtKind::Delete { targets } => {
            for expr in targets {
                visitor.visit_expr(expr);
            }
        }
        StmtKind::Assign { targets, value, .. } => {
            visitor.visit_expr(value);
            for expr in targets {
                visitor.visit_expr(expr);
            }
        }
        StmtKind::AugAssign { target, op, value } => {
            visitor.visit_expr(target);
            visitor.visit_operator(op);
            visitor.visit_expr(value);
        }
        StmtKind::AnnAssign {
            target,
            annotation,
            value,
            ..
        } => {
            visitor.visit_annotation(annotation);
            if let Some(expr) = value {
                visitor.visit_expr(expr);
            }
            visitor.visit_expr(target);
        }
        StmtKind::For {
            target,
            iter,
            body,
            orelse,
            ..
        } => {
            visitor.visit_expr(iter);
            visitor.visit_expr(target);
            visitor.visit_body(body);
            if let Some(orelse) = orelse {
                visitor.visit_body(orelse);
            }
        }
        StmtKind::AsyncFor {
            target,
            iter,
            body,
            orelse,
            ..
        } => {
            visitor.visit_expr(iter);
            visitor.visit_expr(target);
            visitor.visit_body(body);
            if let Some(orelse) = orelse {
                visitor.visit_body(orelse);
            }
        }
        StmtKind::While { test, body, orelse } => {
            visitor.visit_expr(test);
            visitor.visit_body(body);
            if let Some(orelse) = orelse {
                visitor.visit_body(orelse);
            }
        }
        StmtKind::If {
            test, body, orelse, ..
        } => {
            visitor.visit_expr(test);
            visitor.visit_body(body);
            if let Some(orelse) = orelse {
                visitor.visit_body(orelse);
            }
        }
        StmtKind::With { items, body, .. } => {
            for withitem in items {
                visitor.visit_withitem(withitem);
            }
            visitor.visit_body(body);
        }
        StmtKind::AsyncWith { items, body, .. } => {
            for withitem in items {
                visitor.visit_withitem(withitem);
            }
            visitor.visit_body(body);
        }
        StmtKind::Match { subject, cases } => {
            visitor.visit_expr(subject);
            for match_case in cases {
                visitor.visit_match_case(match_case);
            }
        }
        StmtKind::Raise { exc, cause } => {
            if let Some(expr) = exc {
                visitor.visit_expr(expr);
            };
            if let Some(expr) = cause {
                visitor.visit_expr(expr);
            };
        }
        StmtKind::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            visitor.visit_body(body);
            for excepthandler in handlers {
                visitor.visit_excepthandler(excepthandler);
            }
            if let Some(orelse) = orelse {
                visitor.visit_body(orelse);
            }
            if let Some(finalbody) = finalbody {
                visitor.visit_body(finalbody);
            }
        }
        StmtKind::TryStar {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            visitor.visit_body(body);
            for excepthandler in handlers {
                visitor.visit_excepthandler(excepthandler);
            }
            if let Some(orelse) = orelse {
                visitor.visit_body(orelse);
            }
            if let Some(finalbody) = finalbody {
                visitor.visit_body(finalbody);
            }
        }
        StmtKind::Assert { test, msg } => {
            visitor.visit_expr(test);
            if let Some(expr) = msg {
                visitor.visit_expr(expr);
            }
        }
        StmtKind::Import { names } => {
            for alias in names {
                visitor.visit_alias(alias);
            }
        }
        StmtKind::ImportFrom { names, .. } => {
            for alias in names {
                visitor.visit_alias(alias);
            }
        }
        StmtKind::Global { .. } => {}
        StmtKind::Nonlocal { .. } => {}
        StmtKind::Expr { value } => visitor.visit_expr(value),
        StmtKind::Pass => {}
        StmtKind::Break => {}
        StmtKind::Continue => {}
    }
}

pub(crate) fn walk_expr<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, expr: &'a mut Expr) {
    match &mut expr.node {
        ExprKind::BoolOp { ops, values } => {
            for op in ops {
                visitor.visit_bool_op(op);
            }
            for value in values {
                visitor.visit_expr(value);
            }
        }
        ExprKind::NamedExpr { target, value } => {
            visitor.visit_expr(target);
            visitor.visit_expr(value);
        }
        ExprKind::BinOp { left, op, right } => {
            visitor.visit_expr(left);
            visitor.visit_operator(op);
            visitor.visit_expr(right);
        }
        ExprKind::UnaryOp { op, operand } => {
            visitor.visit_unary_op(op);
            visitor.visit_expr(operand);
        }
        ExprKind::Lambda { args, body } => {
            visitor.visit_arguments(args);
            visitor.visit_expr(body);
        }
        ExprKind::IfExp { test, body, orelse } => {
            visitor.visit_expr(test);
            visitor.visit_expr(body);
            visitor.visit_expr(orelse);
        }
        ExprKind::Dict { keys, values } => {
            for expr in keys.iter_mut().flatten() {
                visitor.visit_expr(expr);
            }
            for expr in values {
                visitor.visit_expr(expr);
            }
        }
        ExprKind::Set { elts } => {
            for expr in elts {
                visitor.visit_expr(expr);
            }
        }
        ExprKind::ListComp { elt, generators } => {
            for comprehension in generators {
                visitor.visit_comprehension(comprehension);
            }
            visitor.visit_expr(elt);
        }
        ExprKind::SetComp { elt, generators } => {
            for comprehension in generators {
                visitor.visit_comprehension(comprehension);
            }
            visitor.visit_expr(elt);
        }
        ExprKind::DictComp {
            key,
            value,
            generators,
        } => {
            for comprehension in generators {
                visitor.visit_comprehension(comprehension);
            }
            visitor.visit_expr(key);
            visitor.visit_expr(value);
        }
        ExprKind::GeneratorExp { elt, generators } => {
            for comprehension in generators {
                visitor.visit_comprehension(comprehension);
            }
            visitor.visit_expr(elt);
        }
        ExprKind::Await { value } => visitor.visit_expr(value),
        ExprKind::Yield { value } => {
            if let Some(expr) = value {
                visitor.visit_expr(expr);
            }
        }
        ExprKind::YieldFrom { value } => visitor.visit_expr(value),
        ExprKind::Compare {
            left,
            ops,
            comparators,
        } => {
            visitor.visit_expr(left);
            for cmpop in ops {
                visitor.visit_cmp_op(cmpop);
            }
            for expr in comparators {
                visitor.visit_expr(expr);
            }
        }
        ExprKind::Call {
            func,
            args,
            keywords,
        } => {
            visitor.visit_expr(func);
            for expr in args {
                visitor.visit_expr(expr);
            }
            for keyword in keywords {
                visitor.visit_keyword(keyword);
            }
        }
        ExprKind::FormattedValue {
            value, format_spec, ..
        } => {
            visitor.visit_expr(value);
            if let Some(expr) = format_spec {
                visitor.visit_format_spec(expr);
            }
        }
        ExprKind::JoinedStr { values } => {
            for expr in values {
                visitor.visit_expr(expr);
            }
        }
        ExprKind::Constant { value, .. } => visitor.visit_constant(value),
        ExprKind::Attribute { value, ctx, .. } => {
            visitor.visit_expr(value);
            visitor.visit_expr_context(ctx);
        }
        ExprKind::Subscript { value, slice, ctx } => {
            visitor.visit_expr(value);
            visitor.visit_expr(slice);
            visitor.visit_expr_context(ctx);
        }
        ExprKind::Starred { value, ctx } => {
            visitor.visit_expr(value);
            visitor.visit_expr_context(ctx);
        }
        ExprKind::Name { ctx, .. } => {
            visitor.visit_expr_context(ctx);
        }
        ExprKind::List { elts, ctx } => {
            for expr in elts {
                visitor.visit_expr(expr);
            }
            visitor.visit_expr_context(ctx);
        }
        ExprKind::Tuple { elts, ctx } => {
            for expr in elts {
                visitor.visit_expr(expr);
            }
            visitor.visit_expr_context(ctx);
        }
        ExprKind::Slice { lower, upper, step } => {
            visitor.visit_slice_index(lower);
            visitor.visit_slice_index(upper);
            if let Some(expr) = step {
                visitor.visit_slice_index(expr);
            }
        }
    }
}

pub(crate) fn walk_constant<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    constant: &'a mut Constant,
) {
    if let Constant::Tuple(constants) = constant {
        for constant in constants {
            visitor.visit_constant(constant);
        }
    }
}

pub(crate) fn walk_comprehension<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    comprehension: &'a mut Comprehension,
) {
    visitor.visit_expr(&mut comprehension.iter);
    visitor.visit_expr(&mut comprehension.target);
    for expr in &mut comprehension.ifs {
        visitor.visit_expr(expr);
    }
}

pub(crate) fn walk_excepthandler<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    excepthandler: &'a mut Excepthandler,
) {
    match &mut excepthandler.node {
        ExcepthandlerKind::ExceptHandler { type_, body, .. } => {
            if let Some(expr) = type_ {
                visitor.visit_expr(expr);
            }
            visitor.visit_body(body);
        }
    }
}

pub(crate) fn walk_slice_index<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    slice_index: &'a mut SliceIndex,
) {
    match &mut slice_index.node {
        SliceIndexKind::Empty => {}
        SliceIndexKind::Index { value } => {
            visitor.visit_expr(value);
        }
    }
}

pub(crate) fn walk_arguments<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    arguments: &'a mut Arguments,
) {
    for arg in &mut arguments.posonlyargs {
        visitor.visit_arg(arg);
    }
    for arg in &mut arguments.args {
        visitor.visit_arg(arg);
    }
    if let Some(arg) = &mut arguments.vararg {
        visitor.visit_arg(arg);
    }
    for arg in &mut arguments.kwonlyargs {
        visitor.visit_arg(arg);
    }
    for expr in &mut arguments.kw_defaults {
        visitor.visit_expr(expr);
    }
    if let Some(arg) = &mut arguments.kwarg {
        visitor.visit_arg(arg);
    }
    for expr in &mut arguments.defaults {
        visitor.visit_expr(expr);
    }
}

pub(crate) fn walk_arg<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, arg: &'a mut Arg) {
    if let Some(expr) = &mut arg.node.annotation {
        visitor.visit_annotation(expr);
    }
}

pub(crate) fn walk_keyword<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, keyword: &'a mut Keyword) {
    visitor.visit_expr(&mut keyword.node.value);
}

pub(crate) fn walk_withitem<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    withitem: &'a mut Withitem,
) {
    visitor.visit_expr(&mut withitem.context_expr);
    if let Some(expr) = &mut withitem.optional_vars {
        visitor.visit_expr(expr);
    }
}

pub(crate) fn walk_match_case<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    match_case: &'a mut MatchCase,
) {
    visitor.visit_pattern(&mut match_case.pattern);
    if let Some(expr) = &mut match_case.guard {
        visitor.visit_expr(expr);
    }
    visitor.visit_body(&mut match_case.body);
}

pub(crate) fn walk_pattern<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, pattern: &'a mut Pattern) {
    match &mut pattern.node {
        PatternKind::MatchValue { value } => visitor.visit_expr(value),
        PatternKind::MatchSingleton { value } => visitor.visit_constant(value),
        PatternKind::MatchSequence { patterns } => {
            for pattern in patterns {
                visitor.visit_pattern(pattern);
            }
        }
        PatternKind::MatchMapping { keys, patterns, .. } => {
            for expr in keys {
                visitor.visit_expr(expr);
            }
            for pattern in patterns {
                visitor.visit_pattern(pattern);
            }
        }
        PatternKind::MatchClass {
            cls,
            patterns,
            kwd_patterns,
            ..
        } => {
            visitor.visit_expr(cls);
            for pattern in patterns {
                visitor.visit_pattern(pattern);
            }

            for pattern in kwd_patterns {
                visitor.visit_pattern(pattern);
            }
        }
        PatternKind::MatchStar { .. } => {}
        PatternKind::MatchAs { pattern, .. } => {
            if let Some(pattern) = pattern {
                visitor.visit_pattern(pattern);
            }
        }
        PatternKind::MatchOr { patterns } => {
            for pattern in patterns {
                visitor.visit_pattern(pattern);
            }
        }
    }
}

#[allow(unused_variables)]
pub(crate) fn walk_expr_context<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    expr_context: &'a mut ExprContext,
) {
}

#[allow(unused_variables)]
pub(crate) fn walk_bool_op<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, bool_op: &'a mut BoolOp) {}

#[allow(unused_variables)]
pub(crate) fn walk_operator<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    operator: &'a mut Operator,
) {
}

#[allow(unused_variables)]
pub(crate) fn walk_unary_op<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    unary_op: &'a mut UnaryOp,
) {
}

#[allow(unused_variables)]
pub(crate) fn walk_cmp_op<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, cmp_op: &'a mut CmpOp) {}

#[allow(unused_variables)]
pub(crate) fn walk_alias<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, alias: &'a mut Alias) {}
