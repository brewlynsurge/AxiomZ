use serde::{Serialize, de::DeserializeOwned};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use bincode;


pub async fn send_data<T>(socket: &mut TcpStream, data: &T) -> Result<(), Box<dyn std::error::Error + Send + Sync>> 
where T: Serialize
{
    let encoded = bincode::serialize(data)?;
    // First send lenght of the encoded data
    let len = encoded.len() as u32;
    socket.write_all(&len.to_be_bytes()).await?;
    
    // Send actual bytes
    socket.write_all(&encoded).await?;
    
    Ok(())
}


pub async fn receive_data<T>(socket: &mut TcpStream) -> Result<T, Box<dyn std::error::Error + Send + Sync>> 
where T: DeserializeOwned
{
    // Reading packet lenght
    let mut len_buf = [0u8; 4];
    socket.read_exact(&mut len_buf).await?;
    let packet_len = u32::from_be_bytes(len_buf) as usize;

    // Read packet bytes
    let mut buffer = vec![0u8; packet_len];
    socket.read_exact(&mut buffer).await?;

    // Deserialize data
    let data: T = bincode::deserialize(&buffer)?;
    Ok(data)
}