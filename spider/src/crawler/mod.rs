mod proxy;

pub async fn test() {
    let proxy_rotator = proxy::ProxyRotator::new();
    
    //println!("Validating proxies...");
    //proxy::ProxyRotator::validate_proxies(proxy_rotator.valid_proxies.clone()).await;
    //std::thread::sleep(std::time::Duration::from_secs(5));

    let proxy = proxy_rotator.get_proxy().await;
    println!("Proxy: {}", proxy);
    std::thread::sleep(std::time::Duration::from_secs(20));
}