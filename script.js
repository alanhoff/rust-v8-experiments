// testing timers
const interval = setInterval(() => {
  console.log('Interval');
}, 1000);

setTimeout(() => {
  clearInterval(interval);
  console.log('Interval canceled');
}, 2500);