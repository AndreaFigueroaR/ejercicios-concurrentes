extern crate rand;
extern crate std_semaphore;

use std_semaphore::Semaphore;
use std::thread;
use std::thread::JoinHandle;
use std::sync::Arc;
use std::time::Duration;
use rand::thread_rng;
use rand::Rng;

const CLIENTS: u32 =4;
const INITIAL_CHAIRS: isize = 1;
const INITIAL_CLIENTS: isize = 0;
const INITIAL_HARCUTS_DONE: isize =0;


struct ResourcesSync{
    available_chair:    Arc<Semaphore>,
    client_sitted:      Arc<Semaphore>,
    curret_haircut:     Arc<Semaphore>
}

fn main(){
    
    let available_chair = Arc::new(Semaphore::new(INITIAL_CHAIRS));
    let client_sitted = Arc::new(Semaphore::new(INITIAL_CLIENTS));
    let curret_haircut = Arc::new(Semaphore::new(INITIAL_HARCUTS_DONE));

    let clients_handle: Vec<JoinHandle<_>> =(1..=CLIENTS as u32).map(|id|{
        let local_sync= ResourcesSync{
            available_chair: available_chair.clone(),
            client_sitted:client_sitted.clone(),
            curret_haircut:curret_haircut.clone()
        };
        thread::spawn(move ||client(id, local_sync))
    }).collect();

    barbero(ResourcesSync{available_chair,client_sitted,curret_haircut});

    clients_handle.into_iter().for_each(|t|t.join().unwrap());
    
}

fn barbero(resources_sync: ResourcesSync){
    // como  sé que solo espero 4 clientes en esta prueba cambio loop por un for
    // loop{
    for _ in 0..CLIENTS{
        println!("  [Barbero] Esperando cliente");
        resources_sync.client_sitted.acquire();

        //hacemos el corte de cabello
        println!("[Barbero] Cortando pelo");
        thread::sleep(Duration::from_secs(2));
        println!("[Barbero] Terminé corte de pelo");
        //terminamos corte de cabello
        
        resources_sync.curret_haircut.release();
    }
}

fn client(id:u32,resources_sync: ResourcesSync){
    thread::sleep(Duration::from_secs(thread_rng().gen_range(2..10)));
    println!("[Cliente {}] Entro a la barberia.", id);
    resources_sync.available_chair.acquire();
    resources_sync.client_sitted.release();
    println!("[Cliente {}] Me senté en la silla.", id);

    //esperamos a que nos haga nuestr corte de cabello
    resources_sync.curret_haircut.acquire();
    println!("[Cliente {}] Me terminaron de cortar el cabello.(libero silla)", id);
    //nos vamos, liberamos la silla
    resources_sync.available_chair.release();
}