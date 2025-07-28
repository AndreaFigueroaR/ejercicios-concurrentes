pub trait LeaderElection: Send + Sync {
    fn am_i_leader(&self) -> bool;
    fn get_leader_id(&self) -> usize;
    fn find_new(&mut self);
    fn stop(&mut self);
}
