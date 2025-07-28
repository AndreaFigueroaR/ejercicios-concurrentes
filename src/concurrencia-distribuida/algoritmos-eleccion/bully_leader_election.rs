use crate::leader_election::LeaderElection;
use crate::team_member::{TEAM_MEMBERS, TIMEOUT};
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn id_to_ctrladdr(id: usize) -> String {
    "127.0.0.1:1234".to_owned() + &*id.to_string()
}

pub struct BullyLeaderElection {
    id: usize,
    //socket de control (para administrar la elección) vinculado al puerto 1234 + id
    socket: UdpSocket,
    leader_id: Arc<(Mutex<Option<usize>>, Condvar)>,
    got_ok: Arc<(Mutex<bool>, Condvar)>,
    stop: Arc<(Mutex<bool>, Condvar)>,
}
// Implementa el trait para BullyLeaderElection
impl LeaderElection for BullyLeaderElection {
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
impl BullyLeaderElection {
    pub fn new(id: usize) -> BullyLeaderElection {
        let mut ret = BullyLeaderElection {
            id,
            socket: UdpSocket::bind(id_to_ctrladdr(id)).unwrap(),
            leader_id: Arc::new((Mutex::new(Some(id)), Condvar::new())),
            got_ok: Arc::new((Mutex::new(false), Condvar::new())),
            stop: Arc::new((Mutex::new(false), Condvar::new())),
        };

        let mut clone = ret.clone();
        thread::spawn(move || clone.receiver());

        ret.find_new();
        ret
    }

    pub fn am_i_leader(&self) -> bool {
        self.get_leader_id() == self.id
    }

    pub fn get_leader_id(&self) -> usize {
        self.leader_id
            .1
            .wait_while(self.leader_id.0.lock().unwrap(), |leader_id| {
                leader_id.is_none()
            })
            .unwrap()
            .unwrap()
    }

    // método BLOQUEANTE que envia los mensajes de election y se bloquea hasta que eventualmente el receiver reciba algún OK
    // sino se autoproclama lider
    pub fn find_new(&mut self) {
        if *self.stop.0.lock().unwrap() {
            return;
        }
        if self.leader_id.0.lock().unwrap().is_none() {
            // ya esta buscando lider
            return;
        }
        println!("[{}] buscando lider", self.id);
        *self.got_ok.0.lock().unwrap() = false;
        *self.leader_id.0.lock().unwrap() = None;
        self.send_election();
        let got_ok =
            self.got_ok
                .1
                .wait_timeout_while(self.got_ok.0.lock().unwrap(), TIMEOUT, |got_it| !*got_it);
        if !*got_ok.unwrap().0 {
            self.make_me_leader()
        } else {
            //no necesitamos el acceso que nos dan al recurso, solo queríamos bloquear el hilo que llama a este método
            let _access = self
                .leader_id
                .1
                .wait_while(self.leader_id.0.lock().unwrap(), |leader_id| {
                    leader_id.is_none()
                });
        }
    }

    fn send_election(&self) {
        // P envía el mensaje ELECTION a todos los procesos que tengan número mayor
        let msg = self.id_to_msg(b'E');
        for peer_id in (self.id + 1)..TEAM_MEMBERS {
            self.socket.send_to(&msg, id_to_ctrladdr(peer_id)).unwrap();
        }
    }

    fn make_me_leader(&self) {
        // El nuevo coordinador se anuncia con un mensaje COORDINATOR
        println!("[{}] me anuncio como lider", self.id);
        let msg = self.id_to_msg(b'C');
        for peer_id in 0..TEAM_MEMBERS {
            if peer_id != self.id {
                self.socket.send_to(&msg, id_to_ctrladdr(peer_id)).unwrap();
            }
        }
        *self.leader_id.0.lock().unwrap() = Some(self.id);
        self.leader_id.1.notify_all();
    }

    fn receiver(&mut self) {
        // mientras que no se supona que este proceso (lider) esté caido:
        while !*self.stop.0.lock().unwrap() {
            // recibimos un mensaje (relativo a una eleción) del socket e identificamos el ID del proceso que lo envió
            let mut buf = [0; size_of::<usize>() + 1];
            let (_size, _from) = self.socket.recv_from(&mut buf).unwrap();
            let id_from = usize::from_le_bytes(buf[1..].try_into().unwrap());

            //en caso de que hayamosrecibido el mensaje y se supone que estamos caidos no lo tenemos uqe procesar
            if *self.stop.0.lock().unwrap() {
                break;
            }
            //identificamos el tipo de mensaje
            match &buf[0] {
                // tipo OK (nos estarían garantizando que recursivamente alguien se encargará de solucionar la votación que he de haber inciiado yo)
                b'O' => {
                    //cambiamos el valor de got_ok que posiblemente esté bloqueando el método find_new (al mennos es esperado un rato)
                    // según este se autodeclara inmediatamente lider o se pone a esperar la resolución recursiva esperando que l valor del id del lider sea dif de None
                    *self.got_ok.0.lock().unwrap() = true;
                    self.got_ok.1.notify_all();
                }
                b'E' => {
                    println!("[{}] recibí Election de {}", self.id, id_from);
                    if id_from < self.id {
                        self.socket
                            .send_to(&self.id_to_msg(b'O'), id_to_ctrladdr(id_from))
                            .unwrap();
                        let mut me = self.clone();
                        // NO hacemos el lllamado explícito aquí porque el método es bloqueante hasta que se pueda "leer" que sí se recibió el OK
                        // lo cual debería de suceder en el hilo reciver (Aquí)
                        thread::spawn(move || me.find_new());
                    }
                }
                b'C' => {
                    println!("[{}] recibí nuevo coordinador {}", self.id, id_from);
                    *self.leader_id.0.lock().unwrap() = Some(id_from);
                    self.leader_id.1.notify_all();
                }
                _ => {
                    println!("[{}] ??? {}", self.id, id_from);
                }
            }
        }
        *self.stop.0.lock().unwrap() = false;
        self.stop.1.notify_all();
    }

    pub fn stop(&mut self) {
        *self.stop.0.lock().unwrap() = true;
        let _access = self
            .stop
            .1
            .wait_while(self.stop.0.lock().unwrap(), |should_stop| *should_stop);
    }

    fn clone(&self) -> BullyLeaderElection {
        BullyLeaderElection {
            id: self.id,
            socket: self.socket.try_clone().unwrap(),
            leader_id: self.leader_id.clone(),
            got_ok: self.got_ok.clone(),
            stop: self.stop.clone(),
        }
    }

    fn id_to_msg(&self, header: u8) -> Vec<u8> {
        let mut msg = vec![header];
        msg.extend_from_slice(&self.id.to_le_bytes());
        msg
    }
}
