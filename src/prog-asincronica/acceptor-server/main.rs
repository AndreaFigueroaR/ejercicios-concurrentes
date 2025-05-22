use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
/// Esta es una versión más "verde" dela parte de un programa de unservdior en la que se aceptan conexionestcp y se despachan a un hilo por cliente.
/// Las tareas de tokio hacen mucho más livianos los recursos destinados a cada cliente

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Bindear el listener en la dirección
    let listener = TcpListener::bind("127.0.0.1:12345").await?;
    println!("Esperando conexiones asíncronas...");

    loop {
        // Aceptar nuevas conexiones de forma no bloqueante: me retorna un future
        let (stream, addr) = listener.accept().await?;
        println!("[{}] Cliente conectado", addr);

        // Spawn una tarea ligera que le pasamos al runtime de tokio :D
        tokio::spawn(async move {
            if let Err(e) = process_connection(stream, addr).await {
                panic!("[{}] Error en conexión: {}", addr, e);
            }
        });
    }
}

async fn process_connection(mut stream: TcpStream, addr: std::net::SocketAddr) -> anyhow::Result<()> {
    // separamos el stream (socket conectado) en un lector y un escritor
    let (reader, mut writer) = stream.split();
    
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = buf_reader.read_line(&mut line).await?;
        if bytes == 0 {
            println!("[{}] Cliente desconectado", addr);
            break;
        }

        // Saludooo: echo server
        print!("[{}] Hello {}", addr, line);

        // Enviar respuesta
        writer.write_all(format!("Hello {}", line).as_bytes()).await?;
    }

    Ok(())
}
