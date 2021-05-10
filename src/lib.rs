use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}

enum Job {
    NewJob(Box<dyn FnOnce() + Send>),
    Terminate,
}

pub struct ThreadPool {
    sender: mpsc::Sender<Job>,
    workers: Vec<Worker>,
}

impl ThreadPool {
    pub fn new(thread_num: usize) -> ThreadPool {
        let mut workers = Vec::with_capacity(thread_num);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for _ in 0..thread_num {
            let receiver = receiver.clone();
            workers.push(Worker {
                thread: Some(thread::spawn(move || loop {
                    let msg = receiver.lock().unwrap().recv().unwrap();
                    match msg {
                        Job::NewJob(f) => f(),
                        Job::Terminate => break,
                    }
                })),
            })
        }

        ThreadPool { sender, workers }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender.send(Job::NewJob(Box::new(f))).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in 0..self.workers.len() {
            self.sender.send(Job::Terminate).unwrap()
        }
        self.workers.clear();
    }
}
