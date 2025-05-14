extern crate rand;
use std::thread;
use std::sync::{Arc,RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use rand::{Rng, thread_rng};

const INVERSORES: i32 = 1;
const SALDO_INICIAL: f64 = 100000.0;

/// Se modificó la  consigna para que los inversionistas o tengan que sincrnizarse-> agarran cuando quieren y dejan cuando quieren 
/// auqnue si se agregaron restricciones del cuánto deberían agarrar, inclsuive dependiendo del resultado de **su** inversión previa
fn main(){
    let acceso_saldo = Arc::new(RwLock::new(SALDO_INICIAL));
    // ojo: ya no se necesita tener una nocion de las semanas desde aqui
    // los inversores trabajan independientemente sobre los mismos recursos: pero sin tener que sincronizarse
    let handle_inversores: Vec<JoinHandle<()>> = (0..INVERSORES)
    .map(|id| {
        let acceso_para_inversor = acceso_saldo.clone();
        thread::spawn(move || inversor(id, SALDO_INICIAL / (INVERSORES as f64), acceso_para_inversor))
    }).collect();

    handle_inversores.into_iter()
    .flat_map(|x| x.join())
    .for_each(drop)

}

fn inversor(id:i32, inicial:f64, cuenta:Arc<RwLock<f64>>) {
    let mut capital = inicial;
    while capital > 5.0 {
        println!("[INVERSOR {}] inicio semana {}", id, capital);
        if let Ok(mut saldo) = cuenta.write() {
            *saldo -= capital;
            //quito de la cuenta lo que usaré que inicialmente es lo mismo que se le da a todos
            // luego dependerá del resultado de mi ultima inversión
        }
        
        thread::sleep(Duration::from_millis(1000));
        // del capital dejamos su 90% o 110% (+-10% )
        let resultado = capital * thread_rng().gen_range(0.9..1.1);
        if let Ok(mut money_guard) = cuenta.write() {
            *money_guard += resultado;
        }

        println!("[INVERSOR {}] resultado {}", id, resultado);

        //segun el resultado de esta inversion determinamos el capital para la siguiente inversión
        if resultado > capital {
            capital += (resultado - capital) * 0.5;
        } else {
            capital = resultado;
        }
    }

}