use crate::{
    render_graph::{
        Edge, Node, NodeId, NodeLabel, NodeRunError, NodeState, RenderGraphContext,
        RenderGraphError, SlotInfo, SlotLabel,
    },
    renderer::RenderContext,
};
use bevy_ecs::prelude::World;
use bevy_utils::HashMap;
use std::{borrow::Cow, fmt::Debug};

#[derive(Default)]
pub struct RenderGraph {
    nodes: HashMap<NodeId, NodeState>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
    sub_graphs: HashMap<Cow<'static, str>, RenderGraph>,
    input_node: Option<NodeId>,
}

impl RenderGraph {
    pub const INPUT_NODE_NAME: &'static str = "GraphInputNode";

    pub fn update(&mut self, world: &mut World) {
        for node in self.nodes.values_mut() {
            node.node.update(world);
        }

        for sub_graph in self.sub_graphs.values_mut() {
            sub_graph.update(world);
        }
    }

    pub fn set_input(&mut self, inputs: Vec<SlotInfo>) -> NodeId {
        if self.input_node.is_some() {
            panic!("Graph already has an input node");
        }

        let id = self.add_node("GraphInputNode", GraphInputNode { inputs });
        self.input_node = Some(id);
        id
    }

    #[inline]
    pub fn input_node(&self) -> Option<&NodeState> {
        self.input_node.and_then(|id| self.get_node_state(id).ok())
    }

    pub fn add_node<T>(&mut self, name: impl Into<Cow<'static, str>>, node: T) -> NodeId
    where
        T: Node,
    {
        let id = NodeId::new();
        let name = name.into();
        let mut node_state = NodeState::new(id, node);
        node_state.name = Some(name.clone());
        self.nodes.insert(id, node_state);
        self.node_names.insert(name, id);
        id
    }

    pub fn get_node_state(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<&NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get(&node_id)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    pub fn get_node_state_mut(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get_mut(&node_id)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    pub fn get_node_id(&self, label: impl Into<NodeLabel>) -> Result<NodeId, RenderGraphError> {
        let label = label.into();
        match label {
            NodeLabel::Id(id) => Ok(id),
            NodeLabel::Name(ref name) => self
                .node_names
                .get(name)
                .cloned()
                .ok_or(RenderGraphError::InvalidNode(label)),
        }
    }

    pub fn get_node<T>(&self, label: impl Into<NodeLabel>) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state(label).and_then(|n| n.node())
    }

    pub fn get_node_mut<T>(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state_mut(label).and_then(|n| n.node_mut())
    }

    pub fn add_slot_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        output_slot: impl Into<SlotLabel>,
        input_node: impl Into<NodeLabel>,
        input_slot: impl Into<SlotLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_slot = output_slot.into();
        let input_slot = input_slot.into();
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let output_index = self
            .get_node_state(output_node_id)?
            .output_slots
            .get_slot_index(output_slot.clone())
            .ok_or(RenderGraphError::InvalidOutputNodeSlot(output_slot))?;
        let input_index = self
            .get_node_state(input_node_id)?
            .input_slots
            .get_slot_index(input_slot.clone())
            .ok_or(RenderGraphError::InvalidInputNodeSlot(input_slot))?;

        let edge = Edge::SlotEdge {
            output_node: output_node_id,
            output_index,
            input_node: input_node_id,
            input_index,
        };

        self.validate_edge(&edge)?;

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    pub fn add_node_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        input_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let edge = Edge::NodeEdge {
            output_node: output_node_id,
            input_node: input_node_id,
        };

        self.validate_edge(&edge)?;

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    pub fn validate_edge(&mut self, edge: &Edge) -> Result<(), RenderGraphError> {
        if self.has_edge(edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge.clone()));
        }

        match *edge {
            Edge::SlotEdge {
                output_node,
                output_index,
                input_node,
                input_index,
            } => {
                let output_node_state = self.get_node_state(output_node)?;
                let input_node_state = self.get_node_state(input_node)?;

                let output_slot = output_node_state
                    .output_slots
                    .get_slot(output_index)
                    .ok_or_else(|| {
                        RenderGraphError::InvalidOutputNodeSlot(SlotLabel::Index(output_index))
                    })?;
                let input_slot = input_node_state
                    .input_slots
                    .get_slot(input_index)
                    .ok_or_else(|| {
                        RenderGraphError::InvalidInputNodeSlot(SlotLabel::Index(input_index))
                    })?;

                if let Some(Edge::SlotEdge {
                    output_node: current_output_node,
                    ..
                }) = input_node_state.edges.input_edges.iter().find(|e| {
                    if let Edge::SlotEdge {
                        input_index: current_input_index,
                        ..
                    } = e
                    {
                        input_index == *current_input_index
                    } else {
                        false
                    }
                }) {
                    return Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
                        node: input_node,
                        input_slot: input_index,
                        occupied_by_node: *current_output_node,
                    });
                }

                if output_slot.slot_type != input_slot.slot_type {
                    return Err(RenderGraphError::MismatchedNodeSlots {
                        output_node,
                        output_slot: output_index,
                        input_node,
                        input_slot: input_index,
                    });
                }
            }
            Edge::NodeEdge { .. } => { /* nothing to validate here */ }
        }

        Ok(())
    }

    pub fn has_edge(&self, edge: &Edge) -> bool {
        let output_node_state = self.get_node_state(edge.get_output_node());
        let input_node_state = self.get_node_state(edge.get_input_node());
        if let Ok(output_node_state) = output_node_state {
            if output_node_state.edges.output_edges.contains(edge) {
                if let Ok(input_node_state) = input_node_state {
                    if input_node_state.edges.input_edges.contains(edge) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeState> {
        self.nodes.values()
    }

    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut NodeState> {
        self.nodes.values_mut()
    }

    pub fn iter_node_inputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .input_edges
            .iter()
            .map(|edge| (edge, edge.get_output_node()))
            .map(move |(edge, output_node_id)| {
                (edge, self.get_node_state(output_node_id).unwrap())
            }))
    }

    pub fn iter_node_outputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .output_edges
            .iter()
            .map(|edge| (edge, edge.get_input_node()))
            .map(move |(edge, input_node_id)| (edge, self.get_node_state(input_node_id).unwrap())))
    }

    pub fn add_sub_graph(&mut self, name: impl Into<Cow<'static, str>>, sub_graph: RenderGraph) {
        self.sub_graphs.insert(name.into(), sub_graph);
    }

    pub fn get_sub_graph(&self, name: impl AsRef<str>) -> Option<&RenderGraph> {
        self.sub_graphs.get(name.as_ref())
    }

    pub fn get_sub_graph_mut(&mut self, name: impl AsRef<str>) -> Option<&mut RenderGraph> {
        self.sub_graphs.get_mut(name.as_ref())
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?}", node.id)?;
            writeln!(f, "  in: {:?}", node.input_slots)?;
            writeln!(f, "  out: {:?}", node.output_slots)?;
        }

        Ok(())
    }
}

pub struct GraphInputNode {
    inputs: Vec<SlotInfo>,
}

impl Node for GraphInputNode {
    fn input(&self) -> Vec<SlotInfo> {
        self.inputs.clone()
    }

    fn output(&self) -> Vec<SlotInfo> {
        self.inputs.clone()
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut dyn RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        for i in 0..graph.inputs().len() {
            let input = graph.inputs()[i];
            graph.set_output(i, input)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        render_graph::{
            Edge, Node, NodeId, NodeRunError, RenderGraph, RenderGraphContext, RenderGraphError,
            SlotInfo, SlotType,
        },
        renderer::RenderContext,
    };
    use bevy_ecs::world::World;
    use bevy_utils::HashSet;
    use std::iter::FromIterator;

    #[derive(Debug)]
    struct TestNode {
        inputs: Vec<SlotInfo>,
        outputs: Vec<SlotInfo>,
    }

    impl TestNode {
        pub fn new(inputs: usize, outputs: usize) -> Self {
            TestNode {
                inputs: (0..inputs)
                    .map(|i| SlotInfo::new(format!("in_{}", i), SlotType::TextureView))
                    .collect(),
                outputs: (0..outputs)
                    .map(|i| SlotInfo::new(format!("out_{}", i), SlotType::TextureView))
                    .collect(),
            }
        }
    }

    impl Node for TestNode {
        fn input(&self) -> Vec<SlotInfo> {
            self.inputs.clone()
        }

        fn output(&self) -> Vec<SlotInfo> {
            self.outputs.clone()
        }

        fn run(
            &self,
            _: &mut RenderGraphContext,
            _: &mut dyn RenderContext,
            _: &World,
        ) -> Result<(), NodeRunError> {
            Ok(())
        }
    }

    #[test]
    fn test_graph_edges() {
        let mut graph = RenderGraph::default();
        let a_id = graph.add_node("A", TestNode::new(0, 1));
        let b_id = graph.add_node("B", TestNode::new(0, 1));
        let c_id = graph.add_node("C", TestNode::new(1, 1));
        let d_id = graph.add_node("D", TestNode::new(1, 0));

        graph.add_slot_edge("A", "out_0", "C", "in_0").unwrap();
        graph.add_node_edge("B", "C").unwrap();
        graph.add_slot_edge("C", 0, "D", 0).unwrap();

        fn input_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
            graph
                .iter_node_inputs(name)
                .unwrap()
                .map(|(_edge, node)| node.id)
                .collect::<HashSet<NodeId>>()
        }

        fn output_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
            graph
                .iter_node_outputs(name)
                .unwrap()
                .map(|(_edge, node)| node.id)
                .collect::<HashSet<NodeId>>()
        }

        assert!(input_nodes("A", &graph).is_empty(), "A has no inputs");
        assert!(
            output_nodes("A", &graph) == HashSet::from_iter(vec![c_id]),
            "A outputs to C"
        );

        assert!(input_nodes("B", &graph).is_empty(), "B has no inputs");
        assert!(
            output_nodes("B", &graph) == HashSet::from_iter(vec![c_id]),
            "B outputs to C"
        );

        assert!(
            input_nodes("C", &graph) == HashSet::from_iter(vec![a_id, b_id]),
            "A and B input to C"
        );
        assert!(
            output_nodes("C", &graph) == HashSet::from_iter(vec![d_id]),
            "C outputs to D"
        );

        assert!(
            input_nodes("D", &graph) == HashSet::from_iter(vec![c_id]),
            "C inputs to D"
        );
        assert!(output_nodes("D", &graph).is_empty(), "D has no outputs");
    }

    #[test]
    fn test_get_node_typed() {
        struct MyNode {
            value: usize,
        }

        impl Node for MyNode {
            fn run(
                &self,
                _: &mut RenderGraphContext,
                _: &mut dyn RenderContext,
                _: &World,
            ) -> Result<(), NodeRunError> {
                Ok(())
            }
        }

        let mut graph = RenderGraph::default();

        graph.add_node("A", MyNode { value: 42 });

        let node: &MyNode = graph.get_node("A").unwrap();
        assert_eq!(node.value, 42, "node value matches");

        let result: Result<&TestNode, RenderGraphError> = graph.get_node("A");
        assert_eq!(
            result.unwrap_err(),
            RenderGraphError::WrongNodeType,
            "expect a wrong node type error"
        );
    }

    #[test]
    fn test_slot_already_occupied() {
        let mut graph = RenderGraph::default();

        graph.add_node("A", TestNode::new(0, 1));
        graph.add_node("B", TestNode::new(0, 1));
        graph.add_node("C", TestNode::new(1, 1));

        graph.add_slot_edge("A", 0, "C", 0).unwrap();
        assert_eq!(
            graph.add_slot_edge("B", 0, "C", 0),
            Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
                node: graph.get_node_id("C").unwrap(),
                input_slot: 0,
                occupied_by_node: graph.get_node_id("A").unwrap(),
            }),
            "Adding to a slot that is already occupied should return an error"
        );
    }

    #[test]
    fn test_edge_already_exists() {
        let mut graph = RenderGraph::default();

        graph.add_node("A", TestNode::new(0, 1));
        graph.add_node("B", TestNode::new(1, 0));

        graph.add_slot_edge("A", 0, "B", 0).unwrap();
        assert_eq!(
            graph.add_slot_edge("A", 0, "B", 0),
            Err(RenderGraphError::EdgeAlreadyExists(Edge::SlotEdge {
                output_node: graph.get_node_id("A").unwrap(),
                output_index: 0,
                input_node: graph.get_node_id("B").unwrap(),
                input_index: 0,
            })),
            "Adding to a duplicate edge should return an error"
        );
    }
}
