use super::*;
impl<'a> Engine<'a> {
    fn first(values: Vec<Value>) -> Value {
        values.into_iter().next().unwrap_or(Value::Nil)
    }
    fn number(value: &Value) -> Option<f64> {
        match value {
            Value::Int(v) => Some(*v as f64),
            Value::Float(v) => Some(*v),
            _ => None,
        }
    }
    fn table_id(&self, doc: usize, node: Node<'_>, value: Value) -> Result<usize> {
        match value {
            Value::Table(id) => Ok(id),
            Value::Bytes(_) => self.unknown(
                doc,
                node,
                "String metatable/library indexing is not modeled",
            ),
            Value::Unknown(reason) => self.unknown(doc, node, reason),
            _ => Err(Failure::Lua(Value::Unknown(
                "The exact Lua indexing error text is not modeled",
            ))),
        }
    }
    fn variable(&self, env: &Env, name: &str) -> Result<Value> {
        if let Some(id) = env.locals.get(name) {
            return Ok(self.cells[*id].clone());
        }
        match env.locals.get("_ENV").map(|id| self.cells[*id].clone()) {
            Some(Value::Table(id)) => Ok(self.field(id, name.as_bytes())),
            Some(Value::Unknown(reason)) => Err(Failure::Unknown(0, 0..0, reason.into())),
            _ => Err(Failure::Lua(Value::Unknown(
                "The exact Lua environment-index error text is not modeled",
            ))),
        }
    }
    fn list(
        &mut self,
        doc: usize,
        nodes: Vec<Node<'a>>,
        env: &mut Env,
        depth: usize,
    ) -> Result<Vec<Value>> {
        let last = nodes.len().saturating_sub(1);
        let mut values = vec![];
        for (i, node) in nodes.into_iter().enumerate() {
            let result = self.expr(doc, node, env, depth + 1)?;
            if i == last {
                values.extend(result);
            } else {
                values.push(Self::first(result));
            }
            if values.len() > 1024 {
                return self.budget("Expression list exceeds 1024 results");
            }
        }
        Ok(values)
    }
    fn one(&mut self, doc: usize, node: Node<'a>, env: &mut Env, depth: usize) -> Result<Value> {
        self.expr(doc, node, env, depth).map(Self::first)
    }
    fn required(&self, node: Node<'a>, field: &str) -> Result<Node<'a>> {
        node.child_by_field_name(field)
            .ok_or_else(|| Failure::Budget(format!("Missing parsed {field} field")))
    }
    fn closure(&mut self, doc: usize, node: Node<'a>, env: &Env, method: bool) -> Result<Value> {
        if self.closures.len() >= 1024 {
            return self.budget("Static replay closure limit exceeded");
        }
        let params = self.required(node, "parameters")?;
        let mut names = vec![];
        if method {
            names.push(Arc::from("self"));
        }
        let mut variadic = false;
        for p in named(params) {
            if p.kind() == "identifier" {
                names.push(self.name(text(p, &self.documents[doc].document.source))?);
            } else if p.kind() == "vararg_expression" {
                variadic = true;
            } else {
                return self.unknown(doc, p, "Function parameter attributes are not modeled");
            }
        }
        let captured = self.copied(env)?;
        let id = self.closures.len();
        self.closures.push(Closure {
            document: doc,
            body: node.child_by_field_name("body"),
            params: names,
            env: captured,
            variadic,
        });
        Ok(Value::Closure(id))
    }
    pub(super) fn chunk(
        &mut self,
        doc: usize,
        global: usize,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        let documents = self.documents;
        match &documents[doc].compile {
            Ok(()) => {}
            Err(CompileFailure::Syntax(error)) => {
                return Err(Failure::Lua(Value::Bytes(Arc::from(error.as_slice()))))
            }
            Err(CompileFailure::Budget) => {
                return self.budget("Lua compilation exceeded its allocation budget")
            }
        }
        let tree = documents[doc].tree.as_ref().map_err(|error| {
            Failure::Unknown(
                doc,
                0..documents[doc].document.source.len(),
                format!("Compiled syntax has no supported tree: {error}"),
            )
        })?;
        let mut env = Env {
            variadic: true,
            varargs: args,
            ..Env::default()
        };
        let cell = self.cell(Value::Table(global))?;
        env.locals.insert(Arc::from("_ENV"), cell);
        match self.block(doc, tree.root_node(), &mut env, 0)? {
            Flow::Return(values) => Ok(values),
            Flow::Next => Ok(vec![]),
            Flow::Break => self.unknown(doc, tree.root_node(), "Break outside a modeled loop"),
        }
    }
    fn expr(
        &mut self,
        doc: usize,
        node: Node<'a>,
        env: &mut Env,
        depth: usize,
    ) -> Result<Vec<Value>> {
        self.tick()?;
        if depth > 128 {
            return self.budget("Expression depth exceeds 128");
        }
        let source = &self.documents[doc].document.source;
        let value = match node.kind() {
            "nil" => Value::Nil,
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "number" => match super::super::evaluate(node, source, &BTreeMap::new(), 0) {
                Some(super::super::Expr::Int(v)) => Value::Int(v),
                Some(super::super::Expr::Float(v)) => Value::Float(v),
                _ => {
                    return self.unknown(doc, node, "Numeric literal is outside the current model")
                }
            },
            "string" => {
                let decoded = super::super::decode_string(text(node, source)).ok_or_else(|| {
                    Failure::Unknown(
                        doc,
                        node.byte_range(),
                        "String byte/Unicode escape needs a complete decoder".into(),
                    )
                })?;
                self.bytes(decoded.as_bytes())?
            }
            "identifier" => self.variable(env, text(node, source))?,
            "parenthesized_expression" => self.one(
                doc,
                *named(node)
                    .first()
                    .ok_or_else(|| Failure::Budget("Empty expression".into()))?,
                env,
                depth + 1,
            )?,
            "vararg_expression" => {
                if !env.variadic {
                    return self.unknown(doc, node, "Varargs outside their function");
                }
                return Ok(env.varargs.clone());
            }
            "function_definition" => self.closure(doc, node, env, false)?,
            "dot_index_expression" | "bracket_index_expression" => {
                let object = self.one(doc, self.required(node, "table")?, env, depth + 1)?;
                let id = self.table_id(doc, node, object)?;
                let field = self.required(node, "field")?;
                let key = if node.kind() == "dot_index_expression" {
                    self.bytes(text(field, source).as_bytes())?
                } else {
                    self.one(doc, field, env, depth + 1)?
                };
                if key.is_nil() {
                    Value::Nil
                } else {
                    let key = self.key(key)?;
                    self.get(id, &key)
                }
            }
            "table_constructor" => {
                let id = self.table(false)?;
                let fields = named(node);
                let mut array = 1_i64;
                let mut constructor_keys = BTreeSet::new();
                for (index, field) in fields.iter().copied().enumerate() {
                    let expression = self.required(field, "value")?;
                    let key = if let Some(name) = field.child_by_field_name("name") {
                        if name.kind() == "identifier"
                            && !text(field, source).trim_start().starts_with('[')
                        {
                            let v = self.bytes(text(name, source).as_bytes())?;
                            Some(self.key(v)?)
                        } else {
                            let v = self.one(doc, name, env, depth + 1)?;
                            Some(self.key(v)?)
                        }
                    } else {
                        None
                    };
                    let values = if key.is_none() && index + 1 == fields.len() {
                        self.expr(doc, expression, env, depth + 1)?
                    } else {
                        vec![self.one(doc, expression, env, depth + 1)?]
                    };
                    for value in values {
                        let k = key.clone().unwrap_or_else(|| {
                            let key = Key::Int(array);
                            array += 1;
                            key
                        });
                        if !constructor_keys.insert(k.clone()) {
                            return self.unknown(
                                doc,
                                field,
                                "Duplicate constructor writes have unspecified storage order",
                            );
                        }
                        self.set(id, k, value)?;
                    }
                }
                Value::Table(id)
            }
            "unary_expression" => {
                let value = self.one(doc, self.required(node, "operand")?, env, depth + 1)?;
                let op = text(self.required(node, "operator")?, source);
                match (op, value) {
                    ("not", Value::Unknown(reason)) => return self.unknown(doc, node, reason),
                    ("not", v) => Value::Bool(!v.is_truthy()),
                    ("-", Value::Int(v)) => Value::Int(v.wrapping_neg()),
                    ("-", Value::Float(v)) => Value::Float(-v),
                    ("#", Value::Bytes(v)) => Value::Int(v.len() as i64),
                    ("#", Value::Table(id)) => {
                        if self.tables[id].unknown {
                            return self.unknown(doc, node, "Library table length is not modeled");
                        }
                        let mut n = 0_i64;
                        while self.tables[id].values.contains_key(&Key::Int(n + 1)) {
                            n += 1;
                            self.tick()?;
                        }
                        if self.tables[id]
                            .values
                            .keys()
                            .any(|k| matches!(k,Key::Int(i) if *i>n))
                        {
                            return self.unknown(
                                doc,
                                node,
                                "Length of a table with holes is not inferred",
                            );
                        }
                        Value::Int(n)
                    }
                    _ => {
                        return self.unknown(
                            doc,
                            node,
                            "Unary coercion or metamethod is not modeled",
                        )
                    }
                }
            }
            "binary_expression" => {
                let left = self.one(doc, self.required(node, "left")?, env, depth + 1)?;
                if let Value::Unknown(reason) = left {
                    return self.unknown(doc, node, reason);
                }
                let op = text(self.required(node, "operator")?, source);
                if op == "and" && !left.is_truthy() || op == "or" && left.is_truthy() {
                    left
                } else {
                    let right = self.one(doc, self.required(node, "right")?, env, depth + 1)?;
                    if let Value::Unknown(reason) = right {
                        return self.unknown(doc, node, reason);
                    }
                    self.binary(doc, node, op, left, right)?
                }
            }
            "function_call" => {
                let name = self.required(node, "name")?;
                let mut receiver = None;
                let function = if name.kind() == "method_index_expression" {
                    let object = self.one(doc, self.required(name, "table")?, env, depth + 1)?;
                    let id = self.table_id(doc, name, object.clone())?;
                    let method = text(self.required(name, "method")?, source);
                    receiver = Some(object);
                    self.field(id, method.as_bytes())
                } else {
                    self.one(doc, name, env, depth + 1)?
                };
                let args_node = self.required(node, "arguments")?;
                let mut args = self.list(doc, named(args_node), env, depth + 1)?;
                if let Some(receiver) = receiver {
                    args.insert(0, receiver);
                }
                return self.invoke(doc, node, function, args);
            }
            _ => {
                return self.unknown(
                    doc,
                    node,
                    &format!("Expression {} is not modeled", node.kind()),
                )
            }
        };
        Ok(vec![value])
    }
    fn binary(
        &mut self,
        doc: usize,
        node: Node<'a>,
        op: &str,
        left: Value,
        right: Value,
    ) -> Result<Value> {
        if op == "and" || op == "or" {
            return Ok(right);
        }
        if op == "==" || op == "~=" {
            let same = match (&left, &right) {
                (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
                    b.is_finite()
                        && b.fract() == 0.
                        && *b >= i64::MIN as f64
                        && *b < -(i64::MIN as f64)
                        && *a == *b as i64
                }
                _ => left == right,
            };
            return Ok(Value::Bool(if op == "==" { same } else { !same }));
        }
        if ["<", ">", "<=", ">="].contains(&op) {
            // Converting the integer to double loses order above 2^53. Compare it
            // against the float's floor, after excluding values outside i64.
            let integer_float = |integer: i64, float: f64| {
                use std::cmp::Ordering;
                if float < i64::MIN as f64 {
                    Ordering::Greater
                } else if float >= -(i64::MIN as f64) {
                    Ordering::Less
                } else {
                    match integer.cmp(&(float.floor() as i64)) {
                        Ordering::Equal if float.fract() != 0. => Ordering::Less,
                        order => order,
                    }
                }
            };
            let order = match (&left, &right) {
                (Value::Bytes(a), Value::Bytes(b)) => Some(a.cmp(b)),
                (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
                (Value::Int(a), Value::Float(b)) => Some(integer_float(*a, *b)),
                (Value::Float(a), Value::Int(b)) => Some(integer_float(*b, *a).reverse()),
                _ => Self::number(&left)
                    .zip(Self::number(&right))
                    .and_then(|(a, b)| a.partial_cmp(&b)),
            }
            .ok_or_else(|| {
                Failure::Unknown(
                    doc,
                    node.byte_range(),
                    "Comparison coercion is not modeled".into(),
                )
            })?;
            return Ok(Value::Bool(match op {
                "<" => order.is_lt(),
                ">" => order.is_gt(),
                "<=" => !order.is_gt(),
                _ => !order.is_lt(),
            }));
        }
        if op == ".." {
            let bytes = |v: Value| match v {
                Value::Bytes(v) => Some(v.to_vec()),
                Value::Int(v) => Some(v.to_string().into_bytes()),
                _ => None,
            };
            let mut a = bytes(left).ok_or_else(|| {
                Failure::Unknown(
                    doc,
                    node.byte_range(),
                    "Concatenation coercion is not modeled".into(),
                )
            })?;
            let b = bytes(right).ok_or_else(|| {
                Failure::Unknown(
                    doc,
                    node.byte_range(),
                    "Concatenation coercion is not modeled".into(),
                )
            })?;
            if a.len() + b.len() > 65_536 {
                return self.budget("Concatenation exceeds string limit");
            }
            a.extend(b);
            return self.bytes(&a);
        }
        if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
            let value = match op {
                "+" => Some(a.wrapping_add(*b)),
                "-" => Some(a.wrapping_sub(*b)),
                "*" => Some(a.wrapping_mul(*b)),
                "//" | "%" => {
                    if *b == 0 {
                        return Err(Failure::Lua(Value::Unknown(
                            "The exact Lua arithmetic error text is not modeled",
                        )));
                    }
                    let mut quotient = a.wrapping_div(*b);
                    if a.wrapping_rem(*b) != 0 && (a < &0) != (b < &0) {
                        quotient = quotient.wrapping_sub(1);
                    }
                    Some(if op == "//" {
                        quotient
                    } else {
                        a.wrapping_sub(quotient.wrapping_mul(*b))
                    })
                }
                _ => None,
            };
            if let Some(v) = value {
                return Ok(Value::Int(v));
            }
        }
        let (Some(a), Some(b)) = (Self::number(&left), Self::number(&right)) else {
            return self.unknown(doc, node, "Numeric coercion or metamethod is not modeled");
        };
        let v = match op {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" => a / b,
            "//" => (a / b).floor(),
            "%" => a - (a / b).floor() * b,
            "^" => a.powf(b),
            _ => return self.unknown(doc, node, "Binary operator is not modeled"),
        };
        if !v.is_finite() {
            return self.unknown(
                doc,
                node,
                "Non-finite intermediate numbers need an explicit numeric profile",
            );
        }
        Ok(Value::Float(v))
    }
    fn place(&mut self, doc: usize, node: Node<'a>, env: &mut Env) -> Result<Place> {
        let source = &self.documents[doc].document.source;
        if node.kind() == "identifier" {
            let name = text(node, source);
            if let Some(id) = env.locals.get(name) {
                return Ok(Place::Cell(*id));
            }
            let global = self.variable(env, "_ENV")?;
            let table = self.table_id(doc, node, global)?;
            return Ok(Place::Field(table, Key::Bytes(Arc::from(name.as_bytes()))));
        }
        if [
            "dot_index_expression",
            "bracket_index_expression",
            "method_index_expression",
        ]
        .contains(&node.kind())
        {
            let value = self.one(doc, self.required(node, "table")?, env, 0)?;
            let table = self.table_id(doc, node, value)?;
            let field = self.required(
                node,
                if node.kind() == "method_index_expression" {
                    "method"
                } else {
                    "field"
                },
            )?;
            let value = if node.kind() == "bracket_index_expression" {
                self.one(doc, field, env, 0)?
            } else {
                self.bytes(text(field, source).as_bytes())?
            };
            return Ok(Place::Field(table, self.key(value)?));
        }
        self.unknown(doc, node, "Assignment target is not modeled")
    }
    fn store(&mut self, doc: usize, node: Node<'a>, place: Place, value: Value) -> Result<()> {
        match place {
            Place::Cell(id) => self.cells[id] = value,
            Place::Field(id, key) => {
                if id == self.package
                    && key == Key::Bytes(Arc::from(b"searchers".as_slice()))
                    && value != Value::Table(self.searchers)
                {
                    return self.unknown(
                        doc,
                        node,
                        "Replacing package.searchers needs a modeled searcher pipeline",
                    );
                }
                self.set(id, key, value)?;
            }
        }
        Ok(())
    }
    fn statement(
        &mut self,
        doc: usize,
        node: Node<'a>,
        env: &mut Env,
        depth: usize,
    ) -> Result<Flow> {
        self.tick()?;
        if depth > 64 {
            return self.budget("Statement/block depth exceeds 64");
        }
        let source = &self.documents[doc].document.source;
        match node.kind() {
            "comment" | "hash_bang_line" | "empty_statement" => {}
            "function_call" => {
                self.expr(doc, node, env, 0)?;
            }
            "do_statement" => {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut inner = self.copied(env)?;
                    return self.block(doc, body, &mut inner, depth + 1);
                }
            }
            "return_statement" => {
                let values = named(node)
                    .into_iter()
                    .find(|n| n.kind() == "expression_list")
                    .map(named)
                    .unwrap_or_default();
                return self.list(doc, values, env, 0).map(Flow::Return);
            }
            "break_statement" => return Ok(Flow::Break),
            "variable_declaration" | "assignment_statement" => {
                let local = node.kind() == "variable_declaration";
                let assignment = if local {
                    named(node)
                        .into_iter()
                        .find(|n| n.kind() == "assignment_statement")
                        .unwrap_or(node)
                } else {
                    node
                };
                let children = named(assignment);
                let vars = children
                    .iter()
                    .find(|n| n.kind() == "variable_list")
                    .copied()
                    .ok_or_else(|| Failure::Budget("Missing variable list".into()))?;
                let vars = named(vars);
                let mut places = vec![];
                if local {
                    if vars.iter().any(|v| v.kind() != "identifier") {
                        return self.unknown(
                            doc,
                            node,
                            "Local attributes need modeled close/const semantics",
                        );
                    }
                } else {
                    for var in &vars {
                        let place = self.place(doc, *var, env)?;
                        if places.contains(&place) {
                            return self.unknown(doc,node,"Repeated assignment addresses need an explicit Lua storage-order model");
                        }
                        places.push(place);
                    }
                }
                let rhs = children
                    .iter()
                    .find(|n| n.kind() == "expression_list")
                    .map(|n| named(*n))
                    .unwrap_or_default();
                let values = self.list(doc, rhs, env, 0)?;
                for (i, var) in vars.into_iter().enumerate() {
                    let value = values.get(i).cloned().unwrap_or(Value::Nil);
                    if local {
                        let id = self.cell(value)?;
                        env.locals.insert(self.name(text(var, source))?, id);
                    } else {
                        let place = places.remove(0);
                        self.store(doc, var, place, value)?;
                    }
                }
            }
            "function_declaration" => {
                let name = self.required(node, "name")?;
                let local = node.child(0).is_some_and(|token| token.kind() == "local");
                let place = if local {
                    let id = self.cell(Value::Nil)?;
                    env.locals.insert(self.name(text(name, source))?, id);
                    Place::Cell(id)
                } else {
                    self.place(doc, name, env)?
                };
                let value =
                    self.closure(doc, node, env, name.kind() == "method_index_expression")?;
                self.store(doc, name, place, value)?;
            }
            "if_statement" => {
                let condition = self.one(doc, self.required(node, "condition")?, env, 0)?;
                if let Value::Unknown(reason) = condition {
                    return self.unknown(doc, node, reason);
                }
                let mut body = if condition.is_truthy() {
                    node.child_by_field_name("consequence")
                } else {
                    None
                };
                if !condition.is_truthy() {
                    let mut cursor = node.walk();
                    for branch in node.children_by_field_name("alternative", &mut cursor) {
                        if branch.kind() == "else_statement" {
                            body = branch.child_by_field_name("body");
                            break;
                        }
                        let condition =
                            self.one(doc, self.required(branch, "condition")?, env, 0)?;
                        if let Value::Unknown(reason) = condition {
                            return self.unknown(doc, branch, reason);
                        }
                        if condition.is_truthy() {
                            body = branch.child_by_field_name("consequence");
                            break;
                        }
                    }
                }
                if let Some(body) = body {
                    let mut inner = self.copied(env)?;
                    return self.block(doc, body, &mut inner, depth + 1);
                }
            }
            "while_statement" | "repeat_statement" => loop {
                self.tick()?;
                let mut inner = self.copied(env)?;
                if node.kind() == "while_statement" {
                    let value = self.one(doc, self.required(node, "condition")?, env, 0)?;
                    if let Value::Unknown(reason) = value {
                        return self.unknown(doc, node, reason);
                    }
                    if !value.is_truthy() {
                        break;
                    }
                }
                if let Some(body) = node.child_by_field_name("body") {
                    match self.block(doc, body, &mut inner, depth + 1)? {
                        Flow::Next => {}
                        Flow::Break => break,
                        flow => return Ok(flow),
                    }
                }
                if node.kind() == "repeat_statement" {
                    let value = self.one(doc, self.required(node, "condition")?, &mut inner, 0)?;
                    if let Value::Unknown(reason) = value {
                        return self.unknown(doc, node, reason);
                    }
                    if value.is_truthy() {
                        break;
                    }
                }
            },
            "for_statement" => {
                let clause = self.required(node, "clause")?;
                if clause.kind() != "for_numeric_clause" {
                    return self.unknown(doc, node, "Generic iteration order is not modeled");
                }
                let start = self.one(doc, self.required(clause, "start")?, env, 0)?;
                let end = self.one(doc, self.required(clause, "end")?, env, 0)?;
                let step = if let Some(step) = clause.child_by_field_name("step") {
                    self.one(doc, step, env, 0)?
                } else {
                    Value::Int(1)
                };
                let (Value::Int(mut current), Value::Int(end), Value::Int(step)) =
                    (start, end, step)
                else {
                    return self.unknown(
                        doc,
                        node,
                        "Non-integer numeric loops need a complete coercion profile",
                    );
                };
                if step == 0 {
                    return Err(Failure::Lua(Value::Unknown(
                        "The exact Lua numeric-for error text is not modeled",
                    )));
                }
                while if step > 0 {
                    current <= end
                } else {
                    current >= end
                } {
                    self.tick()?;
                    let mut inner = self.copied(env)?;
                    let id = self.cell(Value::Int(current))?;
                    inner
                        .locals
                        .insert(self.name(text(self.required(clause, "name")?, source))?, id);
                    if let Some(body) = node.child_by_field_name("body") {
                        match self.block(doc, body, &mut inner, depth + 1)? {
                            Flow::Next => {}
                            Flow::Break => break,
                            flow => return Ok(flow),
                        }
                    }
                    let Some(next) = current.checked_add(step) else {
                        break;
                    };
                    current = next;
                }
            }
            _ => {
                return self.unknown(
                    doc,
                    node,
                    &format!("Statement {} is not modeled", node.kind()),
                )
            }
        }
        Ok(Flow::Next)
    }
    fn block(&mut self, doc: usize, node: Node<'a>, env: &mut Env, depth: usize) -> Result<Flow> {
        for statement in named(node) {
            match self.statement(doc, statement, env, depth)? {
                Flow::Next => {}
                flow => return Ok(flow),
            }
        }
        Ok(Flow::Next)
    }
    fn invoke(
        &mut self,
        doc: usize,
        node: Node<'a>,
        function: Value,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        self.calls += 1;
        if self.calls > 32 {
            self.calls -= 1;
            return self.budget("Static replay call depth exceeds 32");
        }
        let result = self.apply(doc, node, function, args);
        self.calls -= 1;
        result
    }
    fn apply(
        &mut self,
        doc: usize,
        node: Node<'a>,
        function: Value,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        self.tick()?;
        match function {
            Value::Closure(id) => {
                let closure = &self.closures[id];
                let (target, body, params, variadic, captured) = (
                    closure.document,
                    closure.body,
                    closure.params.clone(),
                    closure.variadic,
                    closure.env.clone(),
                );
                let mut env = self.copied(&captured)?;
                env.variadic = variadic;
                env.varargs = if variadic {
                    args.get(params.len()..).unwrap_or_default().to_vec()
                } else {
                    vec![]
                };
                for (i, name) in params.into_iter().enumerate() {
                    let id = self.cell(args.get(i).cloned().unwrap_or(Value::Nil))?;
                    env.locals.insert(name, id);
                }
                if let Some(body) = body {
                    match self.block(target, body, &mut env, 0)? {
                        Flow::Next => Ok(vec![]),
                        Flow::Return(v) => Ok(v),
                        Flow::Break => self.unknown(target, body, "Break outside a loop"),
                    }
                } else {
                    Ok(vec![])
                }
            }
            Value::Native(Native::Observe(api)) => {
                if self.events.len() >= 2048 {
                    return self.budget("Replay call observation limit exceeded");
                }
                self.output = self.output.saturating_add(1024);
                let observed_args = if api == "emit" {
                    &args[..args.len().min(1)]
                } else {
                    &args
                };
                let arguments = self.observed(observed_args)?;
                self.events.push(Event {
                    order: self.order,
                    document: self.documents[doc].document.id.clone(),
                    span: node.byte_range(),
                    api: api.into(),
                    arguments,
                });
                self.order += 1;
                Ok(vec![])
            }
            Value::Native(Native::Require(safe)) => self.require(doc, node, args, safe),
            Value::Native(Native::Dofile) | Value::Native(Native::Loadfile) => self.file_call(
                doc,
                node,
                args,
                matches!(function, Value::Native(Native::Loadfile)),
            ),
            Value::Native(Native::Error) => self.raise(doc, node, args, true),
            Value::Native(Native::Pcall) => {
                let f = args.first().cloned().unwrap_or(Value::Nil);
                let args = args.get(1..).unwrap_or_default().to_vec();
                // error() called directly by pcall has a C caller, so level one
                // supplies no Lua source location. A Lua closure does have one.
                let result = if f == Value::Native(Native::Error) {
                    self.raise(doc, node, args, false)
                } else {
                    self.invoke(doc, node, f, args)
                };
                match result {
                    Ok(values) => {
                        let mut result = vec![Value::Bool(true)];
                        result.extend(values);
                        Ok(result)
                    }
                    Err(Failure::Lua(value)) => Ok(vec![Value::Bool(false), value]),
                    Err(other) => Err(other),
                }
            }
            Value::Native(Native::Type) => {
                let name = match args.first().unwrap_or(&Value::Nil) {
                    Value::Nil => "nil",
                    Value::Bool(_) => "boolean",
                    Value::Int(_) | Value::Float(_) => "number",
                    Value::Bytes(_) => "string",
                    Value::Table(_) => "table",
                    Value::Closure(_) | Value::Native(_) => "function",
                    Value::Unknown(reason) => return self.unknown(doc, node, reason),
                };
                self.bytes(name.as_bytes()).map(|v| vec![v])
            }
            Value::Native(Native::Rawget) | Value::Native(Native::Rawset) => {
                let id = self.table_id(doc, node, args.first().cloned().unwrap_or(Value::Nil))?;
                let value = args.get(1).cloned().unwrap_or(Value::Nil);
                if matches!(function, Value::Native(Native::Rawget)) && value.is_nil() {
                    return Ok(vec![Value::Nil]);
                }
                let key = self.key(value)?;
                if matches!(function, Value::Native(Native::Rawget)) {
                    Ok(vec![self.get(id, &key)])
                } else {
                    self.store(
                        doc,
                        node,
                        Place::Field(id, key),
                        args.get(2).cloned().unwrap_or(Value::Nil),
                    )?;
                    Ok(vec![Value::Table(id)])
                }
            }
            Value::Native(Native::Select) => {
                let first = args.first().cloned().unwrap_or(Value::Nil);
                match first {
                    Value::Bytes(b) if b.as_ref() == b"#" => {
                        Ok(vec![Value::Int(args.len().saturating_sub(1) as i64)])
                    }
                    Value::Int(n) if n != 0 => {
                        let count = args.len().saturating_sub(1) as i64;
                        let index = if n < 0 { count + n } else { n - 1 };
                        if index < 0 {
                            return Err(Failure::Lua(Value::Unknown(
                                "The exact Lua select error text is not modeled",
                            )));
                        }
                        Ok(args.get(index as usize + 1..).unwrap_or_default().to_vec())
                    }
                    _ => self.unknown(doc, node, "select argument coercion is not modeled"),
                }
            }
            Value::Native(Native::Unmodeled(name)) => self.unknown(
                doc,
                node,
                &format!("Native operation {name} is not evaluated"),
            ),
            Value::Unknown(reason) => self.unknown(doc, node, reason),
            _ => Err(Failure::Lua(Value::Unknown(
                "The exact Lua call error text is not modeled",
            ))),
        }
    }
    fn raise(
        &mut self,
        doc: usize,
        node: Node<'a>,
        args: Vec<Value>,
        lua_caller: bool,
    ) -> Result<Vec<Value>> {
        let mut value = args.first().cloned().unwrap_or(Value::Nil);
        if let Value::Unknown(reason) = value {
            return self.unknown(doc, node, reason);
        }
        let level = match args.get(1) {
            None | Some(Value::Nil) => 1,
            Some(Value::Int(n)) => *n,
            _ => return self.unknown(doc, node, "Error stack-level coercion is not modeled"),
        };
        if level > 1 {
            return self.unknown(
                doc,
                node,
                "Error stack levels above one need a complete stack model",
            );
        }
        if level > 0 && lua_caller {
            if let Value::Bytes(message) = value {
                let mut full = self.documents[doc].document.load_name.clone();
                if full.len() > 59 {
                    return self.unknown(
                        doc,
                        node,
                        "Long error source-name formatting is not modeled",
                    );
                }
                full.extend(format!(":{}: ", node.start_position().row + 1).bytes());
                full.extend(message.iter());
                value = self.bytes(&full)?;
            }
        }
        Err(Failure::Lua(value))
    }
    fn request(&self, doc: usize, node: Node<'a>, loader: Loader, module: &[u8]) -> Request {
        let package_path = if matches!(loader, Loader::SafeRequire | Loader::OriginalRequire) {
            match self.field(self.package, b"path") {
                Value::Bytes(p) => Some(p.to_vec()),
                _ => None,
            }
        } else {
            None
        };
        Request {
            document: self.documents[doc].document.id.clone(),
            sha256: self.documents[doc].document.sha256.clone(),
            span: node.byte_range(),
            working_directory: self.program.working_directory.clone(),
            loader,
            module: module.to_vec(),
            package_path,
        }
    }
    fn load_event(&mut self, request: Request, outcome: LoadOutcome) -> Result<()> {
        if self.loads.len() >= 512 {
            return self.budget("Replay loader observation limit exceeded");
        }
        let extra = match &outcome {
            LoadOutcome::SuppliedSnapshot {
                document,
                loader_data,
            } => document.len() + loader_data.len(),
            _ => 0,
        };
        let bytes = request.module.len()
            + request.package_path.as_ref().map_or(0, Vec::len)
            + request.working_directory.as_ref().map_or(0, Vec::len)
            + extra;
        self.output = self.output.saturating_add(1024 + bytes.saturating_mul(4));
        if self.output > 2 * MAX_DOCUMENT_BYTES {
            return self.budget("Replay loader observations exceed output budget");
        }
        self.loads.push(LoadEvent {
            order: self.order,
            request,
            outcome,
        });
        self.order += 1;
        Ok(())
    }
    fn resolution(&mut self, request: Request) -> Result<Resolution> {
        let resolution = self
            .program
            .bindings
            .iter()
            .find(|b| b.request == request)
            .map(|b| b.resolution.clone());
        let outcome = match &resolution {
            Some(Resolution::Loaded {
                document,
                loader_data,
            }) => LoadOutcome::SuppliedSnapshot {
                document: document.clone(),
                loader_data: loader_data.clone(),
            },
            Some(Resolution::Missing) => LoadOutcome::SuppliedMissing,
            None => LoadOutcome::NeedsResolution,
        };
        self.load_event(request.clone(), outcome)?;
        resolution.ok_or_else(|| Failure::Pending(Box::new(request)))
    }
    fn require(
        &mut self,
        doc: usize,
        node: Node<'a>,
        args: Vec<Value>,
        safe: bool,
    ) -> Result<Vec<Value>> {
        let Some(Value::Bytes(module)) = args.first() else {
            return self.unknown(doc, node, "Module-name coercion is not modeled");
        };
        if module.len() > 16_384 || module.contains(&0) {
            return self.unknown(doc, node, "Module name exceeds the supported path domain");
        }
        if safe && crate::module_path::is_explicit(module) && crate::module_path::has_glob(module) {
            return self.unknown(
                doc,
                node,
                "Wildcard capture and replay needs ordered glob observations",
            );
        }
        let mut cache = self.cache(module)?;
        if let module_cache::Lookup::Hit(value) =
            cache.regular_lookup(module).map_err(Failure::Budget)?
        {
            let request = self.request(
                doc,
                node,
                if safe {
                    Loader::SafeRequire
                } else {
                    Loader::OriginalRequire
                },
                module,
            );
            self.load_event(request, LoadOutcome::Cached)?;
            return Ok(vec![value]);
        }
        let preload = self.field(self.preload, module);
        let result = if !preload.is_nil() {
            let request = self.request(
                doc,
                node,
                if safe {
                    Loader::SafeRequire
                } else {
                    Loader::OriginalRequire
                },
                module,
            );
            self.load_event(request, LoadOutcome::Preload)?;
            self.invoke(
                doc,
                node,
                preload,
                vec![
                    Value::Bytes(module.clone()),
                    Value::Bytes(Arc::from(b":preload:".as_slice())),
                ],
            )
            .map(|values| (values, Value::Bytes(Arc::from(b":preload:".as_slice()))))
        } else {
            match self.field(self.package, b"path") {
                Value::Bytes(path) if !path.contains(&0) => {}
                Value::Unknown(_) => {}
                _ => {
                    return self.unknown(
                        doc,
                        node,
                        "Invalid package.path needs its actual Lua searcher error semantics",
                    )
                }
            }
            let request = self.request(
                doc,
                node,
                if safe {
                    Loader::SafeRequire
                } else {
                    Loader::OriginalRequire
                },
                module,
            );
            match self.resolution(request)? {
                Resolution::Loaded {
                    document,
                    loader_data,
                } => {
                    let target = self
                        .program
                        .documents
                        .iter()
                        .position(|d| d.id == document)
                        .unwrap();
                    let data = self.bytes(&loader_data)?;
                    self.chunk(
                        target,
                        self.globals,
                        vec![Value::Bytes(module.clone()), data.clone()],
                    )
                    .map(|values| (values, data))
                }
                // The miss itself is supplied, but no actual Lua searcher
                // diagnostic was captured. Its prefix is known to propagate
                // through safe require; never fabricate an observable message.
                Resolution::Missing => {
                    return Err(Failure::Lua(Value::Unknown(
                        "The actual module-not-found diagnostic is not captured",
                    )))
                }
            }
        };
        match result {
            Ok((returns, data)) => {
                cache = self.cache(module)?;
                let value = cache
                    .returned(module, Self::first(returns), Value::Bool(true))
                    .map_err(Failure::Budget)?;
                self.field_set(self.registry, module, value.clone())?;
                Ok(vec![value, data])
            }
            Err(Failure::Lua(error)) if safe => {
                if let Value::Unknown(reason) = error {
                    return self.unknown(doc, node, reason);
                }
                let failure = if let Value::Bytes(message) = &error {
                    module_cache::classify_error(module, message)
                } else {
                    module_cache::Failure::Other
                };
                let table = self.table(false)?;
                cache = self.cache(module)?;
                match cache
                    .failed(
                        module,
                        failure,
                        module_cache::Protection::SafeRequire,
                        || Value::Table(table),
                    )
                    .map_err(Failure::Budget)?
                {
                    module_cache::Failed::Propagate => Err(Failure::Lua(error)),
                    module_cache::Failed::Fallback { value, .. } => {
                        self.logged += 1;
                        if !module.is_empty() {
                            if let Value::Table(package) = self.field(self.globals, b"package") {
                                if let Value::Table(loaded) = self.field(package, b"loaded") {
                                    self.field_set(loaded, module, value.clone())?;
                                }
                            }
                        }
                        Ok(vec![value])
                    }
                }
            }
            Err(other) => Err(other),
        }
    }
    fn file_call(
        &mut self,
        doc: usize,
        node: Node<'a>,
        args: Vec<Value>,
        load: bool,
    ) -> Result<Vec<Value>> {
        let Some(Value::Bytes(path)) = args.first() else {
            return self.unknown(
                doc,
                node,
                "Standard-input loading and filename coercion are not modeled",
            );
        };
        if load {
            match args.get(1) {
                None | Some(Value::Nil) => {}
                Some(Value::Bytes(mode))
                    if [b"t".as_slice(), b"bt", b"tb"].contains(&mode.as_ref()) => {}
                _ => {
                    return self.unknown(
                        doc,
                        node,
                        "loadfile mode is outside the text-source model",
                    )
                }
            }
        }
        let request = self.request(
            doc,
            node,
            if load {
                Loader::Loadfile
            } else {
                Loader::Dofile
            },
            path,
        );
        let target = match self.resolution(request)? {
            Resolution::Loaded { document, .. } => self
                .program
                .documents
                .iter()
                .position(|d| d.id == document)
                .unwrap(),
            Resolution::Missing => {
                let error = Value::Unknown("The actual file-loading diagnostic is not captured");
                return if load {
                    Ok(vec![Value::Nil, error])
                } else {
                    Err(Failure::Lua(error))
                };
            }
        };
        if !load {
            return self.chunk(target, self.globals, vec![]);
        }
        let documents = self.documents;
        match &documents[target].compile {
            Ok(()) => {}
            Err(CompileFailure::Syntax(error)) => return Ok(vec![Value::Nil, self.bytes(error)?]),
            Err(CompileFailure::Budget) => {
                return self.budget("Lua compilation exceeded its allocation budget")
            }
        }
        let tree = documents[target].tree.as_ref().map_err(|error| {
            Failure::Unknown(
                target,
                0..documents[target].document.source.len(),
                format!("Compiled syntax has no supported tree: {error}"),
            )
        })?;
        let environment = args.get(2).cloned().unwrap_or(Value::Table(self.globals));
        let cell = self.cell(environment)?;
        let mut env = Env::default();
        env.locals.insert(Arc::from("_ENV"), cell);
        if self.closures.len() >= 1024 {
            return self.budget("Static replay closure limit exceeded");
        }
        let id = self.closures.len();
        self.closures.push(Closure {
            document: target,
            body: Some(tree.root_node()),
            params: vec![],
            env,
            variadic: true,
        });
        Ok(vec![Value::Closure(id)])
    }
}
