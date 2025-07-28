use crate::leader_election::LeaderElection;
use crate::team_member::{TEAM_MEMBERS, TIMEOUT};

use std::convert::TryInto;
use std::mem::size_of;
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn id_to_ctrladdr(id: usize) -> String {
    "127.0.0.1:1234".to_owned() + &*id.to_string()
}
pub struct RingLeaderElection {
    id: usize,
    socket: UdpSocket,
    leader_id: Arc<(Mutex<Option<usize>>, Condvar)>,
    got_ack: Arc<(Mutex<Option<usize>>, Condvar)>,
    stop: Arc<(Mutex<bool>, Condvar)>,
}

// Implementa el trait para BullyLeaderElection
impl LeaderElection for RingLeaderElection {
    fn am_i_leader(&self) -> bool {
        self.am_i_leader()
    }
    fn get_leader_id(&self) -> usize {
        self.get_leader_id()
    }
    fn find_new(&mut self) {
        self.find_new()
    }
    fn stop(&mut self) {
        self.stop()
    }
}

impl RingLeaderElection {
    pub fn new(id: usize) -> RingLeaderElection {
        let mut ret = RingLeaderElection {
            id,
            socket: UdpSocket::bind(id_to_ctrladdr(id)).unwrap(),
            leader_id: Arc::new((Mutex::new(Some(id)), Condvar::new())),
            got_ack: Arc::new((Mutex::new(None), Condvar::new())),
            stop: Arc::new((Mutex::new(false), Condvar::new())),
        };

        let mut clone = ret.clone();
        thread::spawn(move || clone.receiver());

        ret.find_new();
        ret
    }

    fn am_i_leader(&self) -> bool {
        self.get_leader_id() == self.id
    }

    //método bloqueante hasta que algún proceso de votación haya concluido (nos llegue el mensaje de COORDINATOR)
    fn get_leader_id(&self) -> usize {
        self.leader_id
            .1
            .wait_while(self.leader_id.0.lock().unwrap(), |leader_id| {
                leader_id.is_none()
            })
            .unwrap()
            .unwrap()
    }

    fn find_new(&mut self) {
        if *self.stop.0.lock().unwrap() {
            return;
        }

        println!("[{}] buscando lider", self.id);
        *self.leader_id.0.lock().unwrap() = None;

        //enviamos al siguiente (de forma bloqueante) la lista de ELECTION
        self.safe_send_next(&self.ids_to_msg(b'E', &[self.id]), self.id);

        // usamos la condvar para bloquear el hilo que llama hasta que nos llegue el mensaje de coordinator (al hilo receiver)
        let _access = self
            .leader_id
            .1
            .wait_while(self.leader_id.0.lock().unwrap(), |leader_id| {
                leader_id.is_none()
            });
    }

    // Meétodo bloqueante que envía el mensaje (lista de ids de procesos vivos) al siguiente nodo y
    // espera (bloqueo del hilo actual) hasta que se reciba el ack (lo debe recibir el hilo receiver
    // y cambiar la flag o bien hasta que 'suene' el timeout y se pase a intentar con el siguiente
    fn safe_send_next(&self, msg: &[u8], id: usize) {
        let next_id = self.next(id);
        if next_id == self.id {
            println!("[{}] enviando {} a {}", self.id, msg[0] as char, next_id);
            panic!("Di toda la vuelta sin respuestas")
        }
        *self.got_ack.0.lock().unwrap() = None;
        self.socket.send_to(msg, id_to_ctrladdr(next_id)).unwrap();
        let got_ack =
            self.got_ack
                .1
                .wait_timeout_while(self.got_ack.0.lock().unwrap(), TIMEOUT, |got_it| {
                    got_it.is_none() || got_it.unwrap() != next_id
                });
        if got_ack.unwrap().1.timed_out() {
            self.safe_send_next(msg, next_id)
        }
    }

    fn receiver(&mut self) {
        while !*self.stop.0.lock().unwrap() {
            let mut buf = [0; 1 + size_of::<usize>() + (TEAM_MEMBERS + 1) * size_of::<usize>()];
            let (_size, from) = self.socket.recv_from(&mut buf).unwrap();
            let (msg_type, mut ids) = self.parse_message(&buf);

            match msg_type {
                b'A' => {
                    println!("[{}] recibí ACK de {}", self.id, from);
                    *self.got_ack.0.lock().unwrap() = Some(ids[0]);
                    self.got_ack.1.notify_all();
                }
                b'E' => {
                    println!("[{}] recibí Election de {}, ids {:?}", self.id, from, ids);
                    self.socket
                        .send_to(&self.ids_to_msg(b'A', &[self.id]), from)
                        .unwrap();
                    if ids.contains(&self.id) {
                        // dio toda la vuelta, cambiar a COORDINATOR
                        let winner = *ids.iter().max().unwrap();
                        self.socket
                            .send_to(&self.ids_to_msg(b'C', &[winner, self.id]), from)
                            .unwrap();
                    } else {
                        ids.push(self.id);
                        let msg = self.ids_to_msg(b'E', &ids);
                        let clone = self.clone();
                        //de igual forma que en el algoritmo bully cuando tenemos que 'propagar' la votación lo hacemos en un hilo aparte
                        // pues este método es bloqueante hasta que se reciba el ACK (en el hilo receiver, o sea aquí).
                        thread::spawn(move || clone.safe_send_next(&msg, clone.id));
                    }
                }
                b'C' => {
                    println!(
                        "[{}] recibí nuevo coordinador de {}, ids {:?}",
                        self.id, from, ids
                    );
                    *self.leader_id.0.lock().unwrap() = Some(ids[0]);
                    self.leader_id.1.notify_all();
                    self.socket
                        .send_to(&self.ids_to_msg(b'A', &[self.id]), from)
                        .unwrap();
                    if !ids[1..].contains(&self.id) {
                        ids.push(self.id);
                        let msg = self.ids_to_msg(b'C', &ids);
                        let clone = self.clone();
                        thread::spawn(move || clone.safe_send_next(&msg, clone.id));
                    }
                }
                _ => {
                    println!("[{}] ??? {:?}", self.id, ids);
                }
            }
        }
        *self.stop.0.lock().unwrap() = false;
        self.stop.1.notify_all();
    }

    fn ids_to_msg(&self, header: u8, ids: &[usize]) -> Vec<u8> {
        let mut msg = vec![header];
        msg.extend_from_slice(&ids.len().to_le_bytes());
        for id in ids {
            msg.extend_from_slice(&id.to_le_bytes());
        }
        msg
    }

    fn parse_message(&self, buf: &[u8]) -> (u8, Vec<usize>) {
        let mut ids = vec![];

        let count = usize::from_le_bytes(buf[1..1 + size_of::<usize>()].try_into().unwrap());

        let mut pos = 1 + size_of::<usize>();
        for _id in 0..count {
            ids.push(usize::from_le_bytes(
                buf[pos..pos + size_of::<usize>()].try_into().unwrap(),
            ));
            pos += size_of::<usize>();
        }

        (buf[0], ids)
    }

    fn stop(&mut self) {
        *self.stop.0.lock().unwrap() = true;
        let _access = self
            .stop
            .1
            .wait_while(self.stop.0.lock().unwrap(), |should_stop| *should_stop);
    }

    fn clone(&self) -> RingLeaderElection {
        RingLeaderElection {
            id: self.id,
            socket: self.socket.try_clone().unwrap(),
            leader_id: self.leader_id.clone(),
            got_ack: self.got_ack.clone(),
            stop: self.stop.clone(),
        }
    }

    fn next(&self, id: usize) -> usize {
        (id + 1) % TEAM_MEMBERS
    }
}
