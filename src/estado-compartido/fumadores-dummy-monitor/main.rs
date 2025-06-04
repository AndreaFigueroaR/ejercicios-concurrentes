extern crate rand;

use std::sync::{Arc, Mutex, Condvar};
use std::thread::{self, JoinHandle, Thread};
use rand::{thread_rng, seq::SliceRandom};
//me interesa tener un monitor de la mesa
#[derive(Clone, Copy, Debug)]
enum Ingredients {
    Tobacco = 0,
    Paper,
    Fire
}
struct TableMonitor{
    //como una lista de asistencia de los ingredientes en mesa y un booleano que indica si se puede dejar en la mesa o no
    table_lock: Mutex<(Vec<bool>,bool)>, 
    available_ingredients: Condvar,//para los fumadores
    can_drop: Condvar
}

//defino los metodos con los cuales se interactúa con la mesa.
// Primero el constructor
impl TableMonitor {

    pub fn new()->Self{
        TableMonitor{
            table_lock: Mutex::new((vec![false,false, false],true)),
            available_ingredients: Condvar::new(),
            can_drop: Condvar::new()
        }
    }

    //NOTA: no usamos una referencia muitable porque el acceso a lo que vamos a editar os lo da el mutex 
    // además de que no queremos que serialice el aceso al monitor
    pub fn drop_ingredients(&self, to_add: Vec<Ingredients>){
        println!("[Agente] Esperando a que fumen");
        let mut table_state = 
            self.can_drop.wait_while(self.table_lock.lock().unwrap(), |(table,can_drop)|{
            //espero mientras: haya algun ingrediente en la mesa o no pueda dejar
            table.iter().any(|&ingrediente| ingrediente) || !(*can_drop)
        }).unwrap();

        println!("[Agente] Dejando ingredientes {:?}", to_add);
        //una vez que sí verifiqué que está libre puedo: dejar los ingredientes e indicar que ya n se puede dejar
        for ingredient in to_add{
            table_state.0[ingredient as usize] = true;
        }
        table_state.1 = false;

        //notifico que hay ingredientes disponibles
        self.available_ingredients.notify_all();
    }

    //espero mientras no haya papel y fuego en la mesa
    fn take_complement_to_tobbaco(&self)-> Vec<Ingredients> {
        
        //tomo el lock de la mesa
        let mut table_state = self.available_ingredients.wait_while(self.table_lock.lock().unwrap(), |(table, _)|{
            //pido que haya en la mesa papel y fuego, si no hay entonces espero
            !table[Ingredients::Fire as usize] || !table[Ingredients::Paper as usize]
        }).unwrap();

        //si sí habían los ingredientes los tomo
        table_state.0[Ingredients::Fire as usize] = false;
        table_state.0[Ingredients::Paper as usize] = false;

        vec![Ingredients::Fire, Ingredients::Paper]
        //no notifico a nadie porque eso es en asicrónico
    }

    fn take_complement_to_fire(&self)-> Vec<Ingredients> {
        let mut table_state = self.available_ingredients.wait_while(self.table_lock.lock().unwrap(), |(table, _)|{
            !table[Ingredients::Tobacco as usize] || !table[Ingredients::Paper as usize]
        }).unwrap();

        table_state.0[Ingredients::Tobacco as usize] = false;
        table_state.0[Ingredients::Paper as usize] = false;

        vec![Ingredients::Tobacco, Ingredients::Paper]
    }

    fn take_complement_to_paper(&self)-> Vec<Ingredients> {
        let mut table_state = self.available_ingredients.wait_while(self.table_lock.lock().unwrap(), |(table, _)|{
            //pido que haya en la mesa papel y tobbaco, si no hay entonces espero
            !table[Ingredients::Tobacco as usize] || !table[Ingredients::Fire as usize]
        }).unwrap();

        table_state.0[Ingredients::Tobacco as usize] = false;
        table_state.0[Ingredients::Fire as usize] = false;

        vec![Ingredients::Tobacco, Ingredients::Fire]
        //no notifico a nadie porque eso es en asicrónico
    }

    fn let_agent_drop(&self){
        let mut table_access =self.table_lock.lock().unwrap();
        table_access.1=true;
        self.can_drop.notify_all();
    }
}

pub fn main(){
    let table_monitor = Arc::new(TableMonitor::new());
    let table_monitor_agent = table_monitor.clone();
    //creo hilo para el agente que deja ingredientes
    let agent_handle= thread::spawn(move||{
        //en loop procedo a  dejar ingredentes random en la mesa usando el monitor
        //fabrico un vector deingredientes a dejar y llamo a 
        loop{
            
            //procede a intentar dejar ingredientes
            let mut ings = vec!(Ingredients::Tobacco, Ingredients::Paper, Ingredients::Fire);
            ings.shuffle(&mut thread_rng());
            //debo dejar dos ingredentes
            let selected_ings = &ings[0..ings.len()-1];
            // los dejo mediante el monitor
            table_monitor_agent.drop_ingredients(selected_ings.to_vec());

        }

    });
    //3 fumadores
    let smokers_handles=(0..3).map(|id|{
        let table_monitor_smoker = table_monitor.clone();
        thread::spawn(move || {
            loop {
                //cada fumador espera a que el agente deje ingredientes
                let ingredients = match id {
                    //si el id es 0 suponemos tenía tabaco
                    0 => table_monitor_smoker.take_complement_to_tobbaco(),
                    //si el id es 1 suponemos tenía fuego...
                    1 => table_monitor_smoker.take_complement_to_fire(),
                    _ => table_monitor_smoker.take_complement_to_paper(),
                };
                println!("[Fumador {}] Tomó ingredientes: {:?}", id, ingredients);
                //simula el tiempo de fumar
                println!("[Fumador {}] fumando..", id);
                thread::sleep(std::time::Duration::from_secs(2));
                println!("[Fumador {}] terminó de fumar..", id);
                //una vez que terminó de fumar, le indica al agente que puede dejar más ingredientes
                table_monitor_smoker.let_agent_drop();
            }
        })
    }).collect::<Vec<JoinHandle<_>>>();
    agent_handle.join().unwrap();
    smokers_handles.into_iter().for_each(|t|t.join().unwrap());
}