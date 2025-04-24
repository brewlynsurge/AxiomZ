use std::collections::VecDeque;
use rand::prelude::*;
use rand::distr::weighted::WeightedIndex;

use super::spider::StateMachine;
use super::utils;

pub struct UrlForntier;
impl UrlForntier {
    pub fn append(url: &str, state_machine: &mut StateMachine) -> Result<(), std::io::Error> {
        let url_base = match utils::get_url_base(url) {
            Some(base) => base,
            None => {return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("The url is not valid : {}", url)));}
        };

        if state_machine.urls.contains_key(&url_base) {
            let urls_vec = state_machine.urls.get_mut(&url_base).unwrap();
            if !urls_vec.contains(&url.to_string()) {
                urls_vec.push_back(url.into());
            }
        } else {
            let mut url_vec = VecDeque::new();
            url_vec.push_back(url.to_string());

            state_machine.urls.insert(url_base, url_vec);
        }
        state_machine.total_urls += 1;
        
        Ok(())
    }

    pub fn extend<T>(urls: T, state_machine: &mut StateMachine) where T: IntoIterator<Item = String> {
        for url in urls.into_iter() {
            _ = Self::append(&url, state_machine);
        }
        
    }

    pub fn get_url(state_machine: &mut StateMachine) -> Option<String> {
        let mut urls_vec = Vec::new();
        let mut urls_weights = Vec::new();

        for (key, value) in  state_machine.urls.iter() {
            let avg = value.len().clone() as f32 / state_machine.total_urls as f32;
            urls_vec.push(key.clone());
            urls_weights.push(avg);
        }

        let random_url_base = {
            let dist = match WeightedIndex::new(&urls_weights) {
                Ok(i) => i,
                Err(_) => {return None;}
            };
            
            let mut rng = rand::rng();
            let url_index = dist.sample(&mut rng);
            urls_vec[url_index].clone()
        };
        drop(urls_vec);
        drop(urls_weights);

        let random_url = state_machine.urls.get_mut(&random_url_base).unwrap().pop_front();


        return random_url;
    }
}