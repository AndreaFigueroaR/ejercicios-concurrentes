extern crate rand;

use std::thread::{self, JoinHandle};
use std::sync::{Arc,RwLock};
use std::sync::Barrier;
use std::time::Duration;
use rand::prelude::*;



const INVERSORES: usize = 2;
const SALDO_INICIAL: f64 = 100000.0;
const WEEKNENDS_AMMOUNT: i32 =10;


struct SincInversores{
    inicio_semana :Arc<Barrier>,
    modificar_cuenta : Arc<Barrier>
}

fn main(){
    let cuenta = Arc::new(RwLock::new(SALDO_INICIAL));
    let sinc_lectura = Arc::new(Barrier::new(INVERSORES));
    let sinc_escritura =Arc::new(Barrier::new(INVERSORES));

    let inversores_handle :Vec<JoinHandle<()>>= (0..INVERSORES).map(|id|{
        let acceso_cuenta = cuenta.clone();
        let sinc_local= SincInversores{inicio_semana: sinc_lectura.clone(), modificar_cuenta: sinc_escritura.clone()};
        thread::spawn(move||{inversor(id, acceso_cuenta,sinc_local);})
    }).collect();

    inversores_handle.into_iter().for_each(|h|h.join().unwrap());
}

fn inversor(id:usize, cuenta: Arc<RwLock<f64>>, sinc_inversores: SincInversores){
    for _ in 0..WEEKNENDS_AMMOUNT{
        let mut capital_disp:f64 = 0.0;
        //sincronizo cuando todos están al inicio de la semana listos para leer
        sinc_inversores.inicio_semana.wait();
        if let Ok(saldo_cuenta) = (*cuenta).read(){
            capital_disp= *saldo_cuenta/(INVERSORES as f64);
        }
        //sicronizo cuando todos ya pueden empezar a editar la cuenta
        sinc_inversores.modificar_cuenta.wait();
        println!("[INVERSOR {}] inicio semana con capital disponible {}", id, capital_disp);
        
        if let Ok(mut cuenta_escritura) = cuenta.write(){
            *cuenta_escritura -= capital_disp;//resto con lo que trabajaré
        }//libero cuenta_escritura (se va de scoope)
        
        let result = get_result_inversion(capital_disp);
        println!("[INVERSOR {}] resultado {}", id, result);
        
        if let Ok(mut cuenta_escritura) = cuenta.write(){
            *cuenta_escritura += result;
        } 
    }
}

fn get_result_inversion(inicial_value:f64)->f64{
    let random_result: f64 = rand::thread_rng().gen();
    thread::sleep(Duration::from_millis((random_result * 1000.0) as u64));
    inicial_value * (random_result + 0.5)
}