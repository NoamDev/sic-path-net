use std::{collections::HashMap, fmt::format, str::FromStr, vec};
use hvm64_ast::Tree;

#[derive(Debug)]
pub struct INet {
    pub nodes: Vec<Node>,
    pub free_ports: Vec<Port>,
}

#[derive(Debug,Clone, Copy, Hash, PartialEq, Eq)]
enum Dir {
    Left,
    Right,
}
#[derive(Debug,Clone, Copy, Hash, PartialEq, Eq)]
pub enum AuxPort {
    Free(usize),
    Node{dir: Dir, node: usize}
}

#[derive(Clone,Copy,Debug)]
pub enum Port {
    Principal(usize),
    Aux(AuxPort)
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub label: u16,
    pub principal: AuxPort,
    pub left: Port,
    pub right: Port
}
#[derive(Debug, Clone)]
pub enum Node {
    Era(AuxPort),
    Agent(Agent),
}
struct NameGen {
    id: usize,
}
impl NameGen {
    fn new() -> Self {
        NameGen {
            id: 0
        }
    }
    fn next(&mut self) -> String {
        let mut res = String::new();
        let mut id = self.id;
        while id > 26 {
            let n = (id%26) as u8;
            let c  = (b'a' + n) as char;
            res += &c.to_string();
            id /= 26;
        }
        let n = (id%26) as u8;
        let c  = (b'a' + n) as char;
        res += &c.to_string();

        self.id += 1;
        return res;
    }
}

impl Default for INet {
    fn default() -> Self {
        INet {
            nodes: vec![],
            free_ports: vec![]
        }
    }
}

impl INet {
    fn new() -> Self {
        Default::default()
    }
    pub fn from_str(text: &str) -> Result<INet,String>  {
        let mut net: INet = Default::default();
        let mut wires: HashMap<String, (AuxPort, AuxPort)> = Default::default();
        for (i, line) in text.split('\n').enumerate() {
            if !line.is_empty() {
                let tree = Tree::from_str(line)?;
                net.free_ports.push(Port::Aux(AuxPort::Free(0)));
                add_tree(&mut net, &tree, AuxPort::Free(i), &mut wires)?;
            }
        }
        Ok(net)
    }
    pub fn to_string(&self) -> Result<String,String> {
        let mut res= String::new();
        let mut wires: HashMap<AuxPort, String> = Default::default();
        for port in &self.free_ports {
            let tree = to_tree(port,self, &mut wires, &mut NameGen::new())?;
            res += &tree.to_string();
            res += "\n";
        }
        Ok(res)
    }
    fn port(&self, port: &Port) -> Result<Port, String> {
        match port {
            Port::Principal(node_id) => {
                let node = &self.nodes[node_id.clone()];
                match node {
                    Node::Era(aux_port) => Ok(Port::Aux(aux_port.clone())),
                    Node::Agent(agent) => Ok(Port::Aux(agent.principal.clone())),
                }
            },
            Port::Aux(aux_port) => self.aux_port(aux_port),
        }
    }
    fn set_port(&mut self, key: AuxPort, value: Port) -> Result<Port,String> {
        match key {
            AuxPort::Free(id) => {
                let previous = self.free_ports[id].clone();
                self.free_ports[id] = value;
                Ok(previous)
            },
            AuxPort::Node{dir, node: node_id} => {
                let node = self.nodes[node_id].clone();
                match node {
                    Node::Era(_) => Err("Err does not have aux ports".to_string()),
                    Node::Agent(agent) => {
                        match dir {
                            Dir::Left => {
                                self.nodes[node_id] = Node::Agent(Agent {
                                    label: agent.label,
                                    principal: agent.principal,
                                    left: value,
                                    right: agent.right
                                });
                                Ok(agent.left)
                            },
                            Dir::Right => {
                                self.nodes[node_id] = Node::Agent(Agent {
                                    label: agent.label,
                                    principal: agent.principal,
                                    left: agent.left,
                                    right: value
                                });
                                Ok(agent.right)
                            },
                        }
                       
                    },
                }
            },
        }
    }
    pub fn aux_port(&self, port: &AuxPort) -> Result<Port, String> {
        match port {
            AuxPort::Free(id) => Ok(self.free_ports[id.clone()].clone()),
            AuxPort::Node{dir, node} => {
                let node = &self.nodes[node.clone()];
                match node {
                    Node::Era(_) => Err("Err does not have aux ports".to_string()),
                    Node::Agent(agent) => {
                        match dir {
                            Dir::Left =>  Ok(agent.left.clone()),
                            Dir::Right =>  Ok(agent.right.clone()),
                        }      
                    },
                }
            },
        }
    }
    pub fn node(&self, node_id: usize) -> &Node {
        &self.nodes[node_id]
    }
}

fn add_tree(net: &mut INet, tree: &Tree, parent_port: AuxPort, wires: &mut HashMap<String, (AuxPort, AuxPort)>) -> Result<(), String> {
    match tree {
        Tree::Era => {
            net.nodes.push(Node::Era(parent_port));
            net.set_port(parent_port, Port::Principal(net.nodes.len()-1))?;
            Ok(())
        },
        Tree::Ctr { lab, p1, p2 } => {
            let agent = Agent {
                label: lab.clone(),
                principal: AuxPort::Free(0),
                left: Port::Aux(AuxPort::Free(0)),
                right: Port::Aux(AuxPort::Free(0)),
            };
            net.nodes.push(Node::Agent(agent));
            let node_id  =net.nodes.len()-1; 
            add_tree(net, p1, AuxPort::Node{node: node_id, dir: Dir::Left}, wires)?;
            add_tree(net, p2, AuxPort::Node{node: node_id, dir: Dir::Right}, wires)?;
            net.set_port(parent_port, Port::Principal(node_id))?;
            Ok(())
        },
        Tree::Var(name) => {

            if let Some((port1, port2)) = wires.get_mut(&name.clone()) {
                match port2 {
                    AuxPort::Free(_) => {
                        *port2 = parent_port;
                        net.set_port(parent_port, Port::Aux(port1.clone()))?;
                        net.set_port(port1.clone(), Port::Aux(port2.clone()))?;
                        Ok(())
                    },
                    _ => Err(format!("var {name:?} used more than twice"))
                }
            } else {
                wires.insert(name.clone(), (parent_port, AuxPort::Free(0)));
                Ok(())
            }
        },
        _ => {
            Err("Unsupported node".to_string())
        }
    }
}

fn to_tree(port: &Port, net: &INet, wires: &mut HashMap<AuxPort, String>, gen: &mut NameGen) -> Result<Tree,String> {
    match port {
        Port::Principal(node_id) => {
            let node = &net.nodes[node_id.clone()];
            match node {
                Node::Era(_) => Ok(Tree::Era),
                Node::Agent(agent) => Ok(Tree::Ctr { 
                    lab: agent.label.clone(),
                    p1: Box::new(to_tree(&agent.left, net, wires, gen)?),
                    p2: Box::new(to_tree(&agent.right, net, wires, gen)?),
                 }),
            }
        },
        Port::Aux(aux_port1) => {
            if let Some(name) = wires.get(&aux_port1) {
                Ok(Tree::Var(name.clone()))
            } else {
                let counterpart = net.aux_port(aux_port1)?;
                match counterpart {
                    Port::Principal(_) => Err("aux port points to a principal port".to_string()),
                    Port::Aux(aux_port2) => {
                        let name = gen.next();
                        wires.insert(aux_port1.clone(), name.clone());
                        wires.insert(aux_port2, name.clone());
                        Ok(Tree::Var(name.clone()))
                    },
                }
            }
        }
    }
}

