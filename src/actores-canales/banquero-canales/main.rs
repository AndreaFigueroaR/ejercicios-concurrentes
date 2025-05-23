extern crate rand;

use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use rand::{thread_rng, Rng};
use std::thread::JoinHandle;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

const INVERSORES: i32 = 10;

fn main() {
    let mut plata = 1000.0;
	//  canal que tendrá multiples escritores
    let (devolucion_send, devolucion_receive) = mpsc::channel();
	
    let inversores: Vec<(Sender<f64>, JoinHandle<()>)> = (0..INVERSORES)
        .map(|id| {
		    // creamos un canal con el banquero como unico escritor: por aquí le envío el capital a invertir 
            let (inversor_send, inversor_receive) = mpsc::channel();
            // le creo su copia del canal de múltiples escritores
            let devolucion_send_inversor = devolucion_send.clone();
            // le paso los extremos de canales que necesita
            let t = thread::spawn(move || inversor(id, inversor_receive, devolucion_send_inversor));
            (inversor_send, t)
        })
        .collect();

    // cuerpo de ejecución del banquero:
    loop {
        // primera observación: esto está sucediendo sin que lo "inicie" algún mensaje. NO quedaría así para el modelo de actores
        plata = iniciar_semana(plata, &inversores);

        // llevo una cuenta de cuantos ya me devolvieron el resultado semanal
        let mut devolvieron = HashSet::new();
        while devolvieron.len() < (INVERSORES as usize) {
            let (quien, resultado) = devolucion_receive.recv().unwrap();
            if !devolvieron.contains(&quien) {
                devolvieron.insert(quien);
                // agrego la plata al total
                plata += resultado;
            }
        }
        // recibidos esos N mensajes recién se terminó la semana e inicia la nueva.
        println!("[Banquero] final de semana {}", plata);
    }
    // let _:Vec<()> = inversores.into_iter()
    //     .flat_map(|(_,h)| h.join())
    //     .collect();
}

fn iniciar_semana(capital_semana: f64, inversores: &Vec<(Sender<f64>, JoinHandle<()>)>) -> f64 {
    let capital = capital_semana / (INVERSORES as f64);
    for (inversor, _) in (*inversores).iter() {
        (*inversor).send(capital).unwrap();
    }
    0.0 as f64
}

fn inversor(id: i32, prestamo: Receiver<f64>, devolucion: Sender<(i32, f64)>) {
    loop {
        let plata_inicial = prestamo.recv().unwrap();
        println!("[Inversor {}] me dan {}", id, plata_inicial);
        thread::sleep(Duration::from_secs(2));
        let resultado = plata_inicial * thread_rng().gen_range(0.5..1.5);
        println!("[Inversor {}] devuelvo {}", id, resultado);
        let _ = devolucion.send((id, resultado));
    }
}