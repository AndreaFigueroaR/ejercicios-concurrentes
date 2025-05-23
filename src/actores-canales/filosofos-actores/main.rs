extern crate actix;


use std::collections::{HashMap, HashSet};
use std::time::Duration;


use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, System };
use actix_async_handler::async_handler;
// use actix::clock::sleep;
// use actix::dev::fut::future::Map;
// use actix::fut::result;
use std::collections::VecDeque;


/*****************************************/
/*              DEFINO ACTORES           */
/*****************************************/

/*****************FILOSOFO****************/
struct Philosopher{
    //direcciones a donde pedir 
    chopsticks: HashMap<u32,Addr<Chopstick>>,
    chopsticks_in_hand: HashSet<u32>,
    my_id: u32
}
impl Actor for Philosopher{
    type Context = Context<Self>;
    // tambén tengo inicializar el numero de choksticks que tiene en la mano y los accesos
    // fn started(&mut self, ctx: &mut Self::Context) {
    //     self.chopsticks_in_hand = 0;
    // }
    // esta inicialización la puedo hacer simplemente al crear las instancias de los actores
}

/***************CHOPSTICKS**************/
enum ChopstickState{
    TAKEN,
    RESERVED,
    AVAILABLE
}
struct Chopstick{
    //muts have the queue of the philosophers that are waiting for this chopstick
    waiting_philosophers: VecDeque<Addr<Philosopher>>,
    // also a way to know if the chosktick is inmediatly available, 
    // if it has been taken (and no one is waitning for it) or if 
    // there are more philosophers waiting
    state : ChopstickState,
    id : u32
}
impl Actor for Chopstick{
    type Context = Context<Self>;
    // //me gustaría establecer el estado inicial de los palitos como disponibles
    // fn started(&mut self, ctx: &mut Self::Context) {
    //     self.state = ChopstickState::AVAILABLE;
    // }
    //otra vez, la inicialización la puedo hacer al crear el actor (struct)
}

/*****************************************/
/*         MESSAGES: chopstick           */
/*****************************************/

//**************CHOPSTICK REQUEST**************/
#[derive(Message)]
#[rtype(result = "()")]
struct ChopstickRequest{
    requester: Addr<Philosopher>
}
//handler como no debe "esperar a que pase algo" no es asincronico.
impl Handler<ChopstickRequest> for Chopstick{
    type Result = ();
    fn handle(&mut self, msg: ChopstickRequest, _ctx: &mut Self::Context) -> Self::Result {
        //dependiendo del estado del chopstic se procede:
        match self.state {
            ChopstickState::AVAILABLE => {
                //si el palito está disponible -> se lo doy al filosofo
                self.state = ChopstickState::TAKEN;
                //le aviso al filosofo que ya tiene el palito
                msg.requester.try_send(ChopstickAvailable{chopstick_id: self.id}).unwrap();
            },
            ChopstickState::TAKEN => {
                //si el palito está tomado -> lo reservo para el filosofo que me lo pide
                self.state = ChopstickState::RESERVED;
                self.waiting_philosophers.push_back(msg.requester);
            },
            ChopstickState::RESERVED => {
                //si el palito está reservado -> lo agrego a la cola de espera
                self.waiting_philosophers.push_back(msg.requester);
            }
        }
    }
}

//**************CHOPSTICK RELEASE**************/
#[derive(Message)]
#[rtype(result = "()")]
struct ChopstickRelease{}
impl Handler<ChopstickRelease> for Chopstick{
    type Result = ();
    fn handle(&mut self, _msg: ChopstickRelease, _ctx: &mut Self::Context) -> Self::Result {
        match self.state {
            ChopstickState::AVAILABLE => {
                //shouldnt happen 
                panic!("Chopstick released twice!");
            },
            ChopstickState::TAKEN => {
                //si el palito está tomado -> lo reservo para el filosofo que me lo pide
                self.state = ChopstickState::AVAILABLE;
            },
            ChopstickState::RESERVED => {
                let next_philosopher = self.waiting_philosophers.pop_front();
                match self.waiting_philosophers.len(){
                    0 => {
                        //ya no está reservado sino solo ocupado
                        self.state = ChopstickState::TAKEN;
                        next_philosopher.unwrap().try_send(ChopstickAvailable{chopstick_id: self.id}).unwrap();
                    },
                    _ =>{
                        //si el palito está reservado -> lo reservo para el filosofo que me lo pide
                        self.state = ChopstickState::RESERVED;
                        next_philosopher.unwrap().try_send(ChopstickAvailable{chopstick_id: self.id}).unwrap();
                    }
                }
            }
        }
    }
}


/*****************************************/
/*         MESSAGES: Philosopher         */
/*****************************************/

#[derive(Message)]
#[rtype(result = "()")]
struct Hungry{}
impl Handler<Hungry> for Philosopher{
    type Result = ();
    fn handle(&mut self, _: Hungry, ctx: &mut Self::Context) -> Self::Result {
        println!("[Filósofo {:?}]: Me dio hambre!", self.my_id);
        //cuando un filosofo se siente hambriento pide su primer chopstick
        let request = ChopstickRequest{requester: ctx.address()};
        //busco el chopstick con menor id
        let mut min_chopstick_id:Option<u32>= None;
        for chopstick_id in self.chopsticks.keys(){
            if min_chopstick_id.is_none()|| *chopstick_id < min_chopstick_id.unwrap(){
                min_chopstick_id = Some(*chopstick_id);
            }
        }
        let chopstick_addr= self.chopsticks.get(&min_chopstick_id.unwrap()).unwrap();
        chopstick_addr.try_send(request).unwrap();
        println!("  [Filósofo {:?}]: pedí mi primer palito para comer, de ID {:?}", self.my_id, min_chopstick_id.unwrap());

    }
}


/*CHOPSTICK AVAILABLE: mensaje que el filosofo recibe cuando ya tiene acceso al chopstick */
#[derive(Message)]
#[rtype(result = "()")]
struct ChopstickAvailable{
    //id del chopstick que pidió (para que sepa a quien devolverselo)
    chopstick_id: u32
}

// the philosopher must be capable of handling this message, make this message asyncronic bc it will be sleeping while eating
#[async_handler]
impl Handler<ChopstickAvailable> for Philosopher{
    type Result = ();
    async fn handle(&mut self, msg: ChopstickAvailable, _ctx: &mut Self::Context) -> Self::Result {
        let my_addr = _ctx.address().clone();
        //deoendiendo de cuantos palitos tiene en mano reacciona diferente...
        // si es el primer palito que recibe (solo tiene uno) entonces lo "guarda" y pide el segundo
        match self.chopsticks_in_hand.len(){
            0 => {
                println!("  [Filósofo {:?}]: Recibí mi primer palito para comer", self.my_id);
                //guardo el primer palito (supongo es el de mayor id (derecha digamos)
                self.chopsticks_in_hand.insert(msg.chopstick_id);
                //pido el segundo palito: el que tenga menor id
                let request = ChopstickRequest{requester: my_addr};
                let mut second_chopstick_id:Option<u32>= None;
                for chopstick_id in self.chopsticks.keys(){
                    if second_chopstick_id.is_none()|| *chopstick_id > second_chopstick_id.unwrap(){
                        second_chopstick_id = Some(*chopstick_id);
                    }
                }
                let second_chopstick_addr= self.chopsticks.get(&second_chopstick_id.unwrap()).unwrap();
                second_chopstick_addr.try_send(request).unwrap();
                println!("  [Filósofo {:?}]: pedí mi segundo palito para comer, de ID {:?}", self.my_id, second_chopstick_id.unwrap());

            },
            1 => {
                println!("  [Filósofo {:?}]: Recibí mi segundo palito para comer", self.my_id);
                //ya tenia uno, y me esta llegando la notif de que ya puedo usar el segundo, que debería ser el de menor id
                //guardo el palito
                self.chopsticks_in_hand.insert(msg.chopstick_id);//ojo en algun mometnto tengo que limpiar esto
                my_addr.try_send(Eat{}).unwrap();
            },
            _ => {
                //no debería pasar
                panic!("Philosopher has more than 2 chopsticks");
            }
        }
    }
}

/*FinishedEating: time to release the chopsticks */
#[derive(Message)]
#[rtype(result = "()")]
struct FinishedEating{}
//es un mensaje recursivo que el mismo filosofo se envia al terminar de comer
impl Handler<FinishedEating> for Philosopher{
    type Result = ();
    fn handle(&mut self, _msg: FinishedEating, ctx: &mut Self::Context) -> Self::Result {
        //debo liberar los palitos que tengo en la mano
        for chopstick_id in self.chopsticks_in_hand.iter(){
            let release = ChopstickRelease{};
            let chopstick_addr = self.chopsticks.get(chopstick_id).unwrap();
            //le envio mensaje al chopstick que termine de usarlo
            chopstick_addr.try_send(release).unwrap();
        }
        //limpio los palitos que tengo en la mano
        self.chopsticks_in_hand.clear();
        //me digo a mi mismo que voy a pensar
        ctx.address().try_send(Think{}).unwrap();
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct Think{}
#[async_handler]
impl Handler<Think> for Philosopher{
    type Result = ();
    async fn handle(&mut self, _: Think, _ctx: &mut Self::Context) -> Self::Result {
        //pensamos un toque
        println!("[Filósofo {:?}]: estoy pensando...", self.my_id);
        tokio::time::sleep(Duration::from_secs(2)).await;//por qué aquí si puedo hacer el await?
        //cuando termine de pensar me digo a mi mismo que voy a comer 
        _ctx.address().try_send(Hungry{}).unwrap();

    }

}

#[derive(Message)]
#[rtype(result = "()")]
struct Eat{}
#[async_handler]
impl Handler<Eat> for Philosopher{
    type Result = ();
    async fn handle(&mut self, _: Eat, _ctx: &mut Self::Context) -> Self::Result {
        //comemos un rato
        println!("[Filósofo {:?}]: estoy comiendo...", self.my_id);
        tokio::time::sleep(Duration::from_secs(2)).await;//por qué aquí si puedo hacer el await?
        //cuando termine de pensar me digo a mi mismo que voy a comer 
        _ctx.address().try_send(FinishedEating{}).unwrap();
    }
}



fn main(){
    let system: actix::SystemRunner = System::new();
    system.block_on(async{
        //Lo primero que debo hacer es crear los chopsticks
        let chopsticks = vec![
            Chopstick{id: 0, waiting_philosophers: VecDeque::new(), state: ChopstickState::AVAILABLE}.start(),
            Chopstick{id: 1, waiting_philosophers: VecDeque::new(), state: ChopstickState::AVAILABLE}.start(),
            Chopstick{id: 2, waiting_philosophers: VecDeque::new(), state: ChopstickState::AVAILABLE}.start(),
            Chopstick{id: 3, waiting_philosophers: VecDeque::new(), state: ChopstickState::AVAILABLE}.start(),
            Chopstick{id: 4, waiting_philosophers: VecDeque::new(), state: ChopstickState::AVAILABLE}.start()
        ];
        //luego creo los philosophers
        let philosophers: Vec<Addr<Philosopher>> = (0..5).map(|i_philosopher |{
                let mut chopsticks_access: HashMap<u32, Addr<Chopstick>> = HashMap::new();
                let left_chopstick = i_philosopher;
                let right_chopstick = (i_philosopher+1)%5;
                //agrego los chopsticks que están al lado de este filósofo
                chopsticks_access.insert(left_chopstick,chopsticks[left_chopstick as usize].clone());
                chopsticks_access.insert(right_chopstick,chopsticks[right_chopstick as usize].clone());
                Philosopher{chopsticks:chopsticks_access, chopsticks_in_hand: HashSet::new(), my_id: i_philosopher}.start()
            }).collect::<Vec<_>>();
        println!("Philosophers created");
        //ahora debo crear la tarea inicial que serí decirle aleatoriamente a cada filósofo Hungry o Think
        for i_philosopher in 0..5{
            let philosopher_addr = philosophers[i_philosopher].clone();
            let random_number = rand::random::<u8>()%2;
            if random_number == 0{
                philosopher_addr.send(Hungry{}).await.unwrap();
            } else{
                philosopher_addr.try_send(Think{}).unwrap();
            }
        }

        });
    system.run().unwrap();

}