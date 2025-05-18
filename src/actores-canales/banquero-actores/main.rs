extern crate actix;
extern crate rand;

use std::{collections::HashSet};
use actix::{Actor, Context, Handler, System, Message, Addr};
use rand::{thread_rng, Rng};
use std::time::Duration;
use actix_async_handler::async_handler;
/******************************************/
/*           DEFINICIÓN DE ACTORES        */
/******************************************/
struct Banquero{
    plata: f64,
    inversores: Vec<Addr<Inversor>>
}
struct Inversor{
    id: u16,
    cuenta_destino: Addr<GestionSemanal>
}
struct GestionSemanal {
    inversiones_completadas :HashSet<u16>,
    resultados_acumulados: f64,
    resultados_a_esperar : u16,
    dueño: Addr<Banquero>
}

impl Actor for Banquero{
    type Context = Context<Self>;
}
impl Actor for Inversor{
    type Context = Context<Self>;
}
impl Actor for GestionSemanal{
    type Context = Context<Self>;
}
/******************************************/
/*   DEFINICIÓN DE MENSAJES Y HANDLERS    */
/******************************************/

// menasje para setearle al Banquero el vector de los inversores:
#[derive(Message)]
#[rtype(result = "()")]
struct SetInversores{
    inversores: Vec<Addr<Inversor>>
}

impl Handler<SetInversores> for Banquero{
    type Result = ();
    fn handle(&mut self, msg: SetInversores , _ctx: &mut Context<Self>) -> Self::Result {
        //procedemos a setear los inversores recibidos
        self.inversores = msg.inversores;
    }
}

// -  Normalmente es la cuenta que le indica cuando todas sus inversiones se completaron que invierta otra vez todo (al banquero)
//    pero a un inciio puedes er el main. Esto se realiza a través del mensaje: DistribuirCapitalSemanal
#[derive(Message)]
#[rtype(result = "()")]
struct ResultadoSemanal{resultado_semanal: f64}
// GESTION_SEMANAL -> BANQUERO
impl  Handler<ResultadoSemanal> for Banquero{
    type Result = ();
    fn handle(&mut self, msg: ResultadoSemanal , _ctx: &mut Context<Self>) -> Self::Result {
        self.plata = msg.resultado_semanal;
        println!("[BANQUERO]: inicio semana con {:?}", self.plata);
        if !self.inversores.is_empty(){
            // Aquí clono el vec de recipientes para “liberar” el borrow de &mut self:
            let destinatarios: Vec<_> = self.inversores.clone();
            let capital_inversion = self.plata/(self.inversores.len() as f64 );
            for inversor in destinatarios.into_iter(){
                let capital = CapitalInversion{capital_disponible: capital_inversion};
                inversor.try_send(capital).unwrap();
                //ahora no esperamos a que envie por lo tanto este handler no tiene por qué ser asincrónico
                // si aqui ya terminé -> jamás espero a por la proxima semana. Ya retorno mi valor
            }
        }
    }
}

// -  Además tenemos el mensaje mediante el cual, cuando el banquero debe distribuir su capital: este envia
//    a todos los inversores para que distribuyan su capital.
#[derive(Message)]
#[rtype(result = "()")]
struct CapitalInversion{
    capital_disponible: f64
}
//como este mensaje sí tardará en procesarse lo hago asincrónico: porque maneja futuros
#[async_handler]
impl Handler<CapitalInversion> for Inversor {
    type Result = ();

    fn handle(&mut self, msg: CapitalInversion , _ctx: &mut Context<Self>) -> Self::Result {
        println!("[Inversor {}] me dan {}", self.id, msg.capital_disponible);
        tokio::time::sleep(Duration::from_secs(2)).await;
        let resultado = msg.capital_disponible * thread_rng().gen_range(0.5..1.5);
        println!("[Inversor {}] devuelvo {}", self.id, resultado);
        self.cuenta_destino.try_send(ResultadoInversion{inversor_id: self.id, resultado}).unwrap();
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ResultadoInversion{
    inversor_id : u16,
    resultado: f64
}
impl Handler<ResultadoInversion> for GestionSemanal{
    type Result = ();
    fn handle(&mut self, msg: ResultadoInversion , _ctx: &mut Context<Self>){
        //cuando llega lo que hago es fijarme si quien me lo da no está contado ya.
        // si no lo está-> lo sumo y agrego su id
        if  !self.inversiones_completadas.contains(&msg.inversor_id){
            self.inversiones_completadas.insert(msg.inversor_id);
            println!("[GESTION INVERSIONES SEMALAES]: Se obtuvo el resultado del inversor {:?}, en total tenemos: {:?}", msg.inversor_id,self.inversiones_completadas.len() );
            self.resultados_acumulados += msg.resultado;
        }
        //si es que con esta inversion ya se completaron las inversiones de la semana-> limipo lo de aqui y le aviso lo obtenidp al banquero
        if self.inversiones_completadas.len() == self.resultados_a_esperar as usize{
            self.inversiones_completadas.clear();
            let resultado_semanal = ResultadoSemanal{resultado_semanal: self.resultados_acumulados};
            self.resultados_acumulados =0.0;
            self.dueño.try_send(resultado_semanal).unwrap();
        }
    }
}
//para solucionar el hecho de que el programa estaba concluyendo prematuramente sin dejar tiempo de ejecución a que el resto
// de mensajes se procesen lo que vamos a ahcer es dejar corriendo por siempre a propósito el runtime de actix. Esto lo logramos
// creando un sistema, diciendole que esjecute primero los envios de mensajes que queriamos para iniciar el resto de mensajes,
// luego forzar a que se mantenga corriendo el programa hasta que se haga el llamado explicito en el programa a: System::current().stop()
// el cual no vamos a hacer, por lo tanto podremos ver al programa ejecutarse indefiniidamente.
fn main(){
    //creación explícita del sistema
    let sys = System::new();

    const INVERSORES: u16 = 10;
    sys.block_on(async {
        //creamos el banquero con capital inicial 1000.0, inicialmente no tiene seteados los inversores, espero poder enviarle un mensaje que setee los inversores
        let addr_banquero = Banquero{plata: 1000.0, inversores: vec![]}.start();

        //creamos el GestionSemanal 
        let cuenta_semanal = GestionSemanal{
            inversiones_completadas: HashSet::new(), 
            resultados_acumulados:0.0,
            resultados_a_esperar: INVERSORES,
            dueño: addr_banquero.clone()
        }.start();
        
        let inversores: Vec<Addr<Inversor>> = (0..INVERSORES).map(|id|{
            //por cada inversionista creo el actor y le doy acceso a la direción destino de las inversiones
            Inversor{id,
                cuenta_destino: cuenta_semanal.clone()
            }.start()
        }).collect();
        addr_banquero.send(SetInversores{inversores}).await.unwrap();
        addr_banquero.send(ResultadoSemanal{resultado_semanal:100.0}).await.unwrap();
    });
    // ¡Ahora sí! Esto mantiene el System corriendo indefinidamente
    sys.run().unwrap();
    
    
}