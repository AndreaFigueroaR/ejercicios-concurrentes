use rand::{thread_rng, Rng};

use crate::bully_leader_election::BullyLeaderElection;
use crate::leader_election::LeaderElection;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

pub const TEAM_MEMBERS: usize = 5;
pub const TIMEOUT: Duration = Duration::from_secs(5);

pub struct TeamMember {
    id: usize,
    //socket de datos vinulado al puerto 1235 + id
    socket: UdpSocket,
    enabled: RwLock<bool>,
}

fn id_to_dataaddr(id: usize) -> String {
    "127.0.0.1:1235".to_owned() + &*id.to_string()
}

impl TeamMember {
    pub fn new(id: usize) -> Arc<Self> {
        let socket = UdpSocket::bind(id_to_dataaddr(id)).unwrap();
        Arc::new(TeamMember {
            id,
            socket,
            enabled: RwLock::new(true),
        })
    }

    pub fn run(self: &Arc<Self>) {
        // lanzamos un thread que se ocupa de recibir los mensajes (para miembros cualquiera serán tareas-pong, para SM serán peticiones-ping)
        let (got_pong, pong): (Sender<SocketAddr>, Receiver<SocketAddr>) = mpsc::channel();
        let this = self.clone();
        thread::spawn(move || this.receiver(got_pong));

        //loop en el cual el team Member va cayéndose y volviendo a levantarse
        loop {
            //OJO: antes de empezar a enviar mensajes de datos (ping o pong) primero instancio el LeaderElection
            //mediante el cual se sabrá quien es el lider
            let mut scrum_master: Box<dyn LeaderElection> =
                Box::new(BullyLeaderElection::new(self.id));

            // flag para simular si se cayó el rpoceso o no (protegida porque el hilo receiver al estar "caido"
            // debe ignorar los mensajes de ping)
            *self.enabled.write().unwrap() = true;

            //loop que representa el tiempo en el que está en funcionamiento el proceso
            loop {
                // si el proceso es lider (método bloqueante que espera a que haya un resultado de la elección)
                // ¿Qué elección está en proceso? ¿Cuándo se inició la elección?
                // se inció dentro del constructor del LeaderElection
                // no hay acciones "activas que hacer" como peticiones de tareas y espera de respuestas.
                // la única lógica que de decisiones activas que queda es la de decidir caerse o no (aleatoriamente)
                if scrum_master.am_i_leader() {
                    thread::sleep(Duration::from_millis(thread_rng().gen_range(5000..10000)));
                    // proceso lider decide tomarse vacaciones (caerse)
                    *self.enabled.write().unwrap() = false;
                    break;
                } else {
                    // si el proceso no es lider hay acciones activas que hacer:

                    // pedirle al SM la tarea a realizar (envío activo, nosotros sabemos aquí cuando queremos pedirlo)
                    // y bueno lo hacemos directamente por aquí
                    let leader_id = scrum_master.get_leader_id();
                    self.socket
                        .send_to("PING".as_bytes(), id_to_dataaddr(leader_id))
                        .unwrap();

                    // hasta aquí ya se supone que envíamos el request y que nuestro hilo reciver está por recibir el pong
                    // si nunca lo recibe nunca nos envía nada por este chanel, entonces aquí tenemos un timeout
                    if let Ok(_addr) = pong.recv_timeout(TIMEOUT) {
                        //si recibimos la tarea (pong), por lo que solo simulamos realizarla
                        thread::sleep(Duration::from_millis(thread_rng().gen_range(1000..3000)));
                    } else {
                        // por simplicidad consideramos que cualquier error necesita un lider nuevo: le pedimos explícitamente
                        // al LeaderElection que encuentre un nuevo lider. No parace ser que se dio cuenta por si solo
                        // DUDA: La entidad LeaderElection se habrá dado cuenta sola de que se cayó el lider? ¿Depende del algoritmo?
                        println!("[{}] SM caido, disparo elección", self.id);
                        scrum_master.find_new()
                    }
                }
            }

            scrum_master.stop();

            thread::sleep(Duration::from_secs(60));
        }
    }

    //hilo que se encarga de recivir y responder (en caso de ser necesario) los mensajes recibidos
    fn receiver(self: &Arc<Self>, got_pong: Sender<SocketAddr>) {
        const PING: [u8; 4] = [b'P', b'I', b'N', b'G'];
        const PONG: [u8; 4] = [b'P', b'O', b'N', b'G'];

        let mut buf = [0; 4];
        loop {
            //loop en el que vamos recibiendo mensajes (data de negocios) de: team members (caso SM) o también
            // podrían ser del SM en caso de miembros ordinarios.

            match self.socket.recv_from(&mut buf) {
                Ok((_size, from)) => match buf {
                    PING => {
                        // SCRUM MASTER LOGIC: answer to ping messages
                        println!("[{}] PING de {}", self.id, from);
                        if *self.enabled.read().unwrap() {
                            self.socket.send_to(&PONG, from).unwrap();
                        } else {
                            println!("[{}] ignorado", self.id)
                        }
                    }
                    PONG => {
                        // COMMON TEAM MEMBER LOGIC: when receiving a pong message only have to "count it" (no need of explicit ignoring)
                        got_pong.send(from).unwrap();
                    }
                    _ => println!(
                        "[{}] mensaje desconocido desde {}: {:?}",
                        self.id, from, buf
                    ),
                },
                Err(e) => println!("[{}] error leyendo socket {}", self.id, e),
            }
        }
    }
}
