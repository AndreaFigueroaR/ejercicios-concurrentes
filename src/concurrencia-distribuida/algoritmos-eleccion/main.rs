mod bully_leader_election;
mod leader_election;
mod team_member;

/*
PROBLEMA INTRODUCTORIO: Problema del SCRUM Team
- Un grupo de desarrolladores de software trabaja en un equipo de pares.
- Dentro del equipo, uno de ellos ejerce el rol de Scrum Master.
- Los desarrolladores solicitan al SM la tarea a realizar y le informan a este cuando terminan.
- Cada cierto tiempo, el Scrum Master se cansa de atender a su equipo y decide tomar unas vacaciones sin previo aviso. Al ser un equipo de pares, cualquier otro desarrollador toma las funciones del SM
*/

use std::env;

use crate::team_member::TeamMember;
fn get_id() -> usize {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Uso: {} <id>", args[0]);
        std::process::exit(1);
    }
    // Recibo id por argumento de línea de comando
    args[1].parse().expect("Couldnt parse id")
}
fn main() {
    let id = get_id();
    // con el ID instanciamos un TeamMember que para su funcionamiento correcto (saber quien es el SM)
    // hace uso de alguna instancia de LeaderElection. Dicho TemMember puede comportarse tanto comoSM como otro team Member ordinario.
    TeamMember::new(id).run();
}
