extern crate rand;

use std::time::Duration;

use std::thread;
use std::sync::{Arc,RwLock};
use rand::prelude::*;
use std::mem::discriminant;

const INVERSORES: i32 = 1;
const SALDO_INICIAL: f64 = 100000.0;
const WEEKNENDS_AMMOUNT: i32 =10;
const DUMMY_AMMOUT: f64 =0.0;

fn main() {

    let mut saldo = SALDO_INICIAL;
    let mut semana = 1;
    //referencias originales para banquero
    let buzon_inversion: Arc<RwLock<Option<f64>>> = Arc::new(RwLock::new(None));
    let buzon_resultado: Arc<RwLock<Option<f64>>> = Arc::new(RwLock::new(None));

    //referencias para el inversor
    let fuente_inv1= buzon_inversion.clone();
    let destino_inv1= buzon_resultado.clone(); 

    let handle_inversor = thread::spawn(||inversor(0, fuente_inv1,destino_inv1));
    
    
    while semana <= WEEKNENDS_AMMOUNT {
        println!("[BANQUERO] semana {}, tengo saldo {}", semana, saldo);
        let saldo_individual = saldo / (INVERSORES as f64);
        // LES DOY LA INVERSION INICIAL
        write_when_expected_bussy_waiting(Some(saldo_individual),None,&buzon_inversion);
        saldo = 0.0;
        // LES PIDO LOS RESULTADOS
        if let Some(found)= write_when_expected_bussy_waiting(None, Some(DUMMY_AMMOUT), &buzon_resultado){
            saldo+=found;
        }
        semana += 1
    }
    handle_inversor.join().unwrap();
}

fn inversor(id: i32, buzon_capital: Arc<RwLock<Option<f64>>>, buzon_resultado: Arc<RwLock<Option<f64>>>){
    for _ in 0..WEEKNENDS_AMMOUNT {
        if let Some(capital) = write_when_expected_bussy_waiting(None, Some(DUMMY_AMMOUT), &buzon_capital){
            println!("[INVERSOR {}] inicio semana con {}", id, capital);
            let resultado = get_result_inversion(capital);
            println!("[INVERSOR {}] resultado {}", id, resultado);
            write_when_expected_bussy_waiting(Some(resultado),None,&buzon_resultado);
        }
    }
}

fn get_result_inversion(inicial_value:f64)->f64{
    let random_result: f64 = rand::thread_rng().gen();
    thread::sleep(Duration::from_millis((random_result * 1000.0) as u64));
    inicial_value * (random_result + 0.5)
}

fn write_when_expected_bussy_waiting(value_to_write:Option<f64>, mailbox_cont_expected:Option<f64>, shared_mailbox:&Arc<RwLock<Option<f64>>>)->Option<f64>{
    loop {
        if let Ok(mut mailbox_access) = shared_mailbox.write() {
            if discriminant(&(*mailbox_access)) == discriminant(&mailbox_cont_expected) {
                let found = mailbox_access.clone();
                *mailbox_access = value_to_write;
                return found;
            }
            thread::sleep(Duration::from_millis(100));
        } else{
            panic!();
        }
    }
}