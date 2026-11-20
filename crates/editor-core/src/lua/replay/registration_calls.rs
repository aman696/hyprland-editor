//! Declaration argument observations for captured multi-file projects. This
//! models registration routing, never native field acceptance or runtime effects.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn dispatcher_constructor(
        &mut self,
        doc: usize,
        node: Node<'a>,
        constructor: String,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        if args.len() > 1 {
            return self.unknown(
                doc,
                node,
                "Dispatcher constructor argument arity is unsupported",
            );
        }
        if self.dispatchers.len() >= 2048 {
            return self.budget("Dispatcher construction limit exceeded");
        }
        let observed = self.observed(&args)?;
        let rendered = observed
            .iter()
            .map(scalar_edit::print)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|reason| Failure::Unknown(doc, node.byte_range(), reason))?;
        let source = format!("hl.bind('Q',hl.dsp.{constructor}({}))", rendered.join(","));
        let parsed = super::super::parse(&source, &self.program.version, &[])
            .map_err(|reason| Failure::Unknown(doc, node.byte_range(), reason))?;
        if parsed.opaque_effects
            || !parsed.diagnostics.is_empty()
            || parsed.bindings.binds.len() != 1
        {
            return self.unknown(
                doc,
                node,
                "Dispatcher arguments cannot be represented by the selected typed constructor",
            );
        }
        let id = self.dispatchers.len();
        // Freeze the observed construction arguments. This is source intent,
        // not a prediction of every native closure's captured runtime values.
        self.output = self.output.saturating_add(
            serde_json::to_vec(&observed)
                .map_err(|_| Failure::Budget("Dispatcher payload size is unavailable".into()))?
                .len(),
        );
        if self.output > 2 * MAX_DOCUMENT_BYTES {
            return self.budget("Dispatcher construction payload budget exceeded");
        }
        self.dispatchers.push((constructor, observed));
        Ok(vec![Value::from(Kind::Dispatcher(id))])
    }
    pub(super) fn record_api(
        &mut self,
        doc: usize,
        node: Node<'a>,
        api: &str,
        args: &[Value],
    ) -> Result<usize> {
        if self.events.len() >= 2048 {
            return self.budget("Replay call observation limit exceeded");
        }
        self.output = self.output.saturating_add(1024);
        let args = if api == "emit" {
            &args[..args.len().min(1)]
        } else {
            args
        };
        let (arguments, argument_sources) = self.observed_sources(args)?;
        let order = self.order;
        self.events.push(Event {
            order,
            document: self.documents[doc].document.id.clone(),
            span: node.byte_range(),
            api: api.into(),
            arguments,
            argument_sources,
            path_prefix: self.path_decisions.len(),
        });
        self.order += 1;
        Ok(order)
    }
    pub(super) fn declaration(
        &mut self,
        doc: usize,
        node: Node<'a>,
        api: &str,
        returns_object: bool,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        self.record_api(doc, node, api, &args)?;
        if returns_object
            && node
                .parent()
                .is_none_or(|parent| !matches!(parent.kind(), "chunk" | "block"))
        {
            // The successful handlers return userdata; error routes may return
            // no values. Neither an invented nil nor a guessed result arity is
            // safe when that result is consumed by the source program.
            return self.unknown(
                doc,
                node,
                "A declaration object's returned values or methods are not modeled",
            );
        }
        Ok(vec![])
    }
    pub(super) fn define_submap(
        &mut self,
        doc: usize,
        node: Node<'a>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        let order = self.record_api(doc, node, "hl.define_submap", &args)?;
        let callback = match args.as_slice() {
            [Value {
                kind: Kind::Bytes(_),
                ..
            }, callback] => callback,
            [Value {
                kind: Kind::Bytes(_),
                ..
            }, Value {
                kind: Kind::Bytes(_),
                ..
            }, callback] => callback,
            _ => return self.unknown(
                doc,
                node,
                "Submap routing needs a captured string name, optional string reset and function",
            ),
        };
        if !matches!(callback.kind, Kind::Closure(_) | Kind::Native(_)) {
            return self.unknown(doc, node, "Submap callback is not a modeled function");
        }
        if self.registration_scopes.len() >= 256 {
            return self.budget("Submap registration scope limit exceeded");
        }
        let index = self.registration_scopes.len();
        self.registration_scopes.push(RegistrationScope {
            declaration_order: order,
            end_order: self.order,
            completed: false,
        });
        // This interprets only the immediate registration callback through the
        // existing bounded AST model. Input-triggered binding/gesture callbacks
        // stay inert, and no imported Lua bytecode is invoked.
        let result = self.invoke(doc, node, callback.clone(), vec![]);
        self.registration_scopes[index].end_order = self.order;
        self.registration_scopes[index].completed = result.is_ok();
        match result {
            Ok(_) => Ok(vec![]),
            Err(Failure::Lua(_)) => self.unknown(
                doc,
                node,
                "A submap callback raised an error; native error recovery remains unresolved",
            ),
            Err(error) => Err(error),
        }
    }
}
