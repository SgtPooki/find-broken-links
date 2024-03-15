use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct DebugSender<T> {
    sender: mpsc::Sender<T>,
    counter: Arc<AtomicUsize>, // Use Arc to share this atomic between multiple senders
    max_size: Arc<AtomicUsize>, // Add this field to DebugSender
}

impl<T> DebugSender<T> {
    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        // Increase the count immediately when `.send` is called
        let current_size = self.counter.fetch_add(1, Ordering::SeqCst) + 1;

        // Check and update max_size if necessary
        let mut max_size = self.max_size.load(Ordering::SeqCst);
        while current_size > max_size {
            match self.max_size.compare_exchange(max_size, current_size, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => break, // Successfully updated max_size
                Err(current) => max_size = current, // max_size was changed by another thread, retry with new value
            }
        }

        // Perform the send operation
        let send_result = self.sender.send(value).await;

        // Decrease the count when the send message finishes, regardless of success or failure
        self.counter.fetch_sub(1, Ordering::SeqCst);

        // Return the result of the send operation
        send_result
    }
}

pub struct DebugChannel<T> {
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
    counter: Arc<AtomicUsize>,
    max_size: Arc<AtomicUsize>,   // Tracks the maximum size the buffer has reached
    _marker: PhantomData<T>, // This is used to associate the generic type T with the struct without storing it
}

impl<T> DebugChannel<T> {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<T>(buffer_size);
        DebugChannel {
            sender,
            receiver,
            counter: Arc::new(AtomicUsize::new(0)),
            max_size: Arc::new(AtomicUsize::new(0)),
            _marker: PhantomData,
        }
    }

    pub fn sender(&self) -> DebugSender<T> {
        DebugSender {
            sender: self.sender.clone(),
            counter: self.counter.clone(), // Clone the Arc to share the counter
            max_size: self.max_size.clone(), // Make sure to clone max_size as well

        }
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }

    // // You can use this method for debugging to get the current number of items in the channel
    // // Method to get current buffer usage
    // pub fn get_current_buffer_usage(&self) -> usize {
    //     self.counter.load(Ordering::SeqCst)
    // }

    // Method to get max buffer size reached (useful after processing to see how full the buffer gets)
    pub fn get_max_buffer_size(&self) -> usize {
        self.max_size.load(Ordering::SeqCst)
    }
}
