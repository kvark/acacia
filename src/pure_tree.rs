//! Pure tree implementation.

use std::mem;
use std::iter::IntoIterator;
use traits::{NodeState, Node, Position};
use partition::{Pack, Container, Partition};
use iter::Iter;


/// A pure N-dimensional tree
pub struct PureTree<P, O> where P: Pack<PureTree<P, O>> {
    state: NodeState<O, Box<<P as Pack<PureTree<P, O>>>::Container>>,
    partition: P,
}

impl<P, O> PureTree<P, O>
    where P: Pack<PureTree<P, O>>,
{
    fn empty(partition: P) -> PureTree<P, O> {
        PureTree {
            state: NodeState::Empty,
            partition: partition,
        }
    }
}

impl<P, O> PureTree<P, O>
    where O: Position,
          P: Partition<<O as Position>::Point> + Pack<PureTree<P, O>>,
{
    /// Construct a tree without checking the geometry of the input data
    pub fn new<I: Iterator<Item=O>>(iter: I, partition: P) -> PureTree<P, O> {
        let mut tree = PureTree::empty(partition);
        for object in iter {
            tree.insert(object)
        }
        tree
    }

    fn dispatch(&self, nodes: &mut <P as Pack<PureTree<P, O>>>::Container, object: O) {
        nodes.index_mut(self.partition.dispatch(&object.position())).insert(object)
    }

    fn insert(&mut self, object: O) {
        let mut tmp = NodeState::Empty;
        mem::swap(&mut tmp, &mut self.state);
        self.state = match tmp {
            NodeState::Empty => NodeState::Leaf(object),
            NodeState::Leaf(other) => {
                let mut nodes = self.partition
                    .pack(|p| PureTree::empty(p));
                self.dispatch(&mut nodes, object);
                self.dispatch(&mut nodes, other);
                NodeState::Branch(Box::new(nodes))
            },
            NodeState::Branch(mut nodes) => {
                self.dispatch(&mut *nodes, object);
                NodeState::Branch(nodes)
            }
        };
    }
}

impl<P, O> Node for PureTree<P, O>
    where P: Clone + Pack<PureTree<P, O>>
{
    type Partition = P;
    type Object = O;
    type Container = <P as Pack<PureTree<P, O>>>::Container;

    fn state(&self) -> NodeState<&O, &<P as Pack<PureTree<P, O>>>::Container> {
        use traits::NodeState::*;
        match self.state {
            Empty => Empty,
            Leaf(ref object) => Leaf(object),
            Branch(ref container) => Branch(&*container),
        }
    }

    fn partition(&self) -> P {
        self.partition.clone()
    }
}

impl<'a, P, O> IntoIterator for &'a PureTree<P, O>
    where P: Clone + Pack<PureTree<P, O>> + 'a,
          O: 'a,
          <P as Pack<PureTree<P, O>>>::Container: 'a,
{
    type Item = &'a O;
    type IntoIter = Iter<'a, PureTree<P, O>>;
    fn into_iter(self) -> Iter<'a, PureTree<P, O>> { Iter::new(self) }
}


#[cfg(test)]
mod test {
    use std::iter::AdditiveIterator;
    use rand::distributions::{IndependentSample, Range};
    use rand::thread_rng;
    use test::Bencher;
    use nalgebra::{Pnt2, FloatPnt, Orig};

    use traits::{Node, ObjectQuery, Positioned};
    use partition::Interval;
    use super::*;

    #[bench]
    fn pure_tree_quad_new_1000(b: &mut Bencher) {
        let coord_dist = Range::new(-1.0, 1.0);
        let mut rng = thread_rng();
        let vec: Vec<_> = (0..1000).map(|_| Positioned {
            object: (),
            position: coord_dist.ind_sample(&mut rng),
        }).collect();
        b.iter(|| {
            PureTree::new(
                vec.iter().map(|a| a.clone()),
                Interval::new(-1.0f64, 1.0),
            )
        })
    }

    #[bench]
    fn pure_tree_query_objects(b: &mut Bencher) {
        let coord_dist = Range::new(-1.0, 1.0);
        let mut rng = thread_rng();
        let search_radius = 0.3;
        let tree = PureTree::new(
            (0..1000).map(|_| Positioned {
                object: (),
                position: coord_dist.ind_sample(&mut rng),
            }),
            Interval::new(-1.0, 1.0),
        );
        b.iter(|| {
            // Count the number of objects within the search radius 10000 times
            (0..10000)
                .map(|_|
                    tree.query_objects(|node|
                        node.partition()
                        .center().dist(&Orig::orig())
                        < search_radius + node.partition().width() / 2.0,
                    )
                    .filter(|other| other.position.dist(&Orig::orig()) < search_radius)
                    .count()
                )
                .sum()
        })
    }
}
