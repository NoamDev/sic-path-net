use std::{collections::HashMap, iter::zip};

use crate::inet::{AuxPort, INet, Node::{Agent, Era}, Port};

#[derive(Debug,Clone, Copy)]
struct PathId(usize);
#[derive(Debug,Clone, Copy)]
struct BranchId(usize);
#[derive(Debug,Clone, Copy,PartialEq,Eq)]
struct AgentPathId(usize);
#[derive(Debug)]
struct Branch {
    left: Option<BranchId>,
    right: Option<BranchId>,
    paths: Vec<AgentPathId>,
}
#[derive(Debug)]
struct AgentPath {
    b1: BranchId,
    b2: BranchId,
    paths: Vec<PathId>,
}
#[derive(Debug)]
struct AgentPNet {
    branches: Vec<Branch>,
    root: BranchId,
    paths: Vec<AgentPath>,
}
#[derive(Debug)]
struct Path<const N: usize> {
    agents: [AgentPathId; N],
}
#[derive(Debug)]
pub struct PNet<const N: usize> {
    agents: [AgentPNet; N],
    paths: Vec<Path<N>>,
}

impl<const N: usize> Default for PNet<N> {
    fn default() -> Self {
        Self {
            agents: core::array::from_fn(|_|Default::default()),
            paths: Default::default(),
        }
    }
}

impl Default for AgentPNet {
    fn default() -> Self {
        Self {
            branches: vec![Default::default()],
            root: BranchId(0),
            paths: Default::default()
        }
    }
}

impl Default for Branch {
    fn default() -> Self {
        Self {
            left: Default::default(),
            right: Default::default(),
            paths: Default::default()
        }
    }
}

fn add_paths<const N: usize>(port: Port, inet: &INet, parents: [BranchId; N], pnet: &mut PNet<N>, paths: &mut HashMap<AuxPort, [BranchId; N]>) -> Result<(),String> {
    match port {
        Port::Principal(node_id) => {
            let node = inet.node(node_id);
            match node {
                Era(_) => Ok(()),
                Agent(agent) => {
                    let parent = parents[agent.label as usize].clone(); 
                    let subnet = &mut pnet.agents[agent.label as usize];
                    let left = subnet.add_left(parent);
                    let right = subnet.add_right(parent);
                    let mut parents_left = parents.clone();
                    let mut parents_right = parents.clone();
                    parents_left[agent.label as usize] = left;
                    parents_right[agent.label as usize] = right;
                    add_paths(agent.left, inet, parents_left, pnet, paths)?;
                    add_paths(agent.right, inet, parents_right, pnet, paths)?;
                    Ok(())
                },
            }
        },
        Port::Aux(aux_port) => {
            if let Some(other_parents) = paths.get(&aux_port) {
                let agent_paths: Vec<AgentPathId> = 
                    zip(parents, other_parents).enumerate()
                    .map(|(i, (b1, b2))| {
                        pnet.agent_mut(i as u16)
                            .add_path(b1, b2.clone())
                    }).collect();
                let path = pnet.add_path(agent_paths.clone().try_into().unwrap());
                for (i,agent_path_id) in agent_paths.iter().enumerate() {
                    pnet.agent_mut(i as u16)
                        .path_mut(agent_path_id.clone())
                        .paths.push(path);
                }
                Ok(())
            } else {
                let other = inet.aux_port(&aux_port)?;
                match other {
                    Port::Principal(_) => Err("Unexpected principal port".to_string()),
                    Port::Aux(other_aux) => {
                        paths.insert(aux_port, parents);
                        paths.insert(other_aux, parents);
                        Ok(())
                    },
                }
            }
        },
    }
}

impl<const N: usize> PNet<N> {
    fn add_path(&mut self, agent_paths: [AgentPathId; N]) -> PathId {
        self.paths.push(Path{
            agents: agent_paths
        });
        PathId(self.paths.len()-1)
    }
    fn path(&self, id: &PathId) -> &Path<N> {
        &self.paths[id.0]
    }
    fn agent(&self, label: u16) -> &AgentPNet {
        &self.agents[label as usize]
    }
    fn agent_mut(&mut self, label: u16) -> &mut AgentPNet {
        &mut self.agents[label as usize]
    }
    fn path_mut(&mut self, id: &PathId) -> &mut Path<N> {
        &mut self.paths[id.0]
    }
    pub fn from_inet(inet: &INet) -> Result<Self, String> {
        let mut res: PNet<N> = Default::default();
        let mut map: HashMap<AuxPort, [BranchId; N]> = Default::default();
        for port in &inet.free_ports {
            add_paths(port.clone(), inet, [BranchId(0); N], &mut res, &mut map)?
        }
        Ok(res)
    }
    fn remove_empty_branches(&mut self) {
        for agent in self.agents.iter_mut() {
            agent.remove_empty_branches();
        }
    }
}

fn remove_empty_branches(agent_pnet: &mut AgentPNet, branch_id: BranchId) -> bool {
    let branch = agent_pnet.branch(&branch_id);
    let left_id = branch.left;
    let right_id = branch.right;
    let has_paths = !branch.paths.is_empty();
    
    let is_left = if let Some(left) = left_id {
        remove_empty_branches(agent_pnet, left)
    } else {
        false
    };
    let is_right = if let Some(right) = right_id {
        remove_empty_branches(agent_pnet, right)
    } else {
        false
    };

    if !is_left {
        let branch = agent_pnet.branch_mut(&branch_id);
        branch.left = None;
    }
    if !is_right {
        let branch = agent_pnet.branch_mut(&branch_id);
        branch.right = None;
    }
    is_left && is_right && has_paths
}

impl AgentPNet {
    fn branch(&self, id: &BranchId) -> &Branch {
        &self.branches[id.0]
    }
    fn branch_mut(&mut self, id: &BranchId) -> &mut Branch {
        &mut self.branches[id.0]
    }
    fn add_left(&mut self, parent_id: BranchId) -> BranchId {
        let parent = &self.branches[parent_id.0];
        if let Some(id) = parent.left {
            id.clone()
        } else {
            let id = BranchId(self.branches.len() - 1);
            let parent = &mut self.branches[parent_id.0];
            parent.left = Some(id);
            self.branches.push(Default::default());
            id
        }
    }
    fn add_right(&mut self, parent_id: BranchId) -> BranchId {
        let parent = &self.branches[parent_id.0];
        if let Some(id) = parent.left {
            id.clone()
        } else {
            let id = BranchId(self.branches.len() - 1);
            let parent = &mut self.branches[parent_id.0];
            parent.right = Some(id);
            self.branches.push(Default::default());
            id
        }
    }
    fn get_path(&self, b1_id: BranchId, b2_id: BranchId) -> Option<AgentPathId> {
        let b1 = self.branch(&b1_id);
        let b2 = self.branch(&b2_id);
        for p1 in b1.paths.iter() {
            for p2 in b2.paths.iter() {
                if p1 == p2 {
                    return Some(p1.clone());
                }
            }    
        }
        None
    }
    fn add_path(&mut self, b1_id: BranchId, b2_id: BranchId) -> AgentPathId {
        if let Some(path_id) = self.get_path(b1_id, b2_id) {
            path_id
        } else {
            let path = AgentPath {
                b1: b1_id,
                b2: b2_id,
                paths: Default::default()
            };
            self.paths.push(path);
            let id = AgentPathId(self.paths.len()-1);
            let b1 = self.branch_mut(&b1_id);
            b1.paths.push(id);
            let b2 = self.branch_mut(&b1_id);
            b2.paths.push(id);
            id
        }
    }

    fn path(&self, agent_path_id: AgentPathId) -> &AgentPath {
        &self.paths[agent_path_id.0]
    }
    fn path_mut(&mut self, agent_path_id: AgentPathId) -> &mut AgentPath {
        &mut self.paths[agent_path_id.0]
    }
    fn remove_empty_branches(&mut self) {
        remove_empty_branches(self, self.root);
    }
}