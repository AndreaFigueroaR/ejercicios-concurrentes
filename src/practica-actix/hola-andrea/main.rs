extern crate actix;

use actix::{Actor, Context, Handler, System, Message};

// este es el primer programa que escirbo usnaod el framework actix
// el cual será usado para modelar los problemas de concurrencia bajo el modelo de actores


/***********************ACTORES************************/
// Primero definimos cuales son los tipos de actores: 
// En este caso quiero un actor que responda al mensaje con mi nombre: Andrea
// Y me responda con un saludo, en síntesis solo quiero un saludador
struct Saludador{} //no necesita por ahora algun estado interno
impl Actor for Saludador{
    type Context =Context<Self>;
}
/***********************MENSAJE************************/
#[derive(Message)]
#[rtype(result = "String")]
struct Saludame{
    mi_nombre: String
}
/*******************HANDLER DEL MENSAJE*****************/
impl Handler<Saludame > for Saludador {
    type Result = String;
    fn handle(&mut self, msg: Saludame , _ctx: &mut Context<Self>) -> Self::Result {
        // le concatenamos el nombre que nos llegó  en el mensaje
        "Hello ".to_owned() + &msg.mi_nombre
    }
}

#[actix_rt::main]
async fn main(){
    //Creamos el actor
    let addr_saludador = Saludador{}.start();
    //creamos el mesaje que le enviaremos y hará que se ejecute su hadler
    let mensaje = Saludame{mi_nombre: String::from("Andrea")};
    //le envio el mensaje y en este caos esperaré por su respuesta
    let resultado = addr_saludador.send(mensaje).await;
    println!("{}", resultado.unwrap());
    System::current().stop();
}