void setup() {
  Serial.begin(115200);
  noInterrupts ();

  // external clock on falling edge on T1 pin (Arduino Nano pin name: D5)
  TCCR1A = 0;
  TCCR1B = 6;
  TCNT1 = 0;

  while (1) {
    uint16_t counter = TCNT1;
    Serial.println(counter);
  }
}
