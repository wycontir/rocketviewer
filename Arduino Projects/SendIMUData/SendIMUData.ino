#include <Arduino.h>
#include <Adafruit_BNO08x.h>

#define BNO08X_RESET -1
Adafruit_BNO08x bno08x(BNO08X_RESET);

sh2_SensorValue_t sensorValue;
// 250Hz more accurate report
sh2_SensorId_t reportType = SH2_ARVR_STABILIZED_RV;
long reportIntervalUs = 5000;

// {"x":0.17,"y":0.00,"z":0.18,"w":0.97,"time":30280}
// {"x":0.00,"y":0.23,"z":0.00,"w":0.97,"time":14802}
//sensor quat values
float x = 0.0;
float y = 0.0;
float z = 0.0;
float w = 0.0;

void setup() {
  Serial.begin(9600);
  while(!Serial) delay(10); //wait until serial console opens
  
  //Serial.println("Begin BNO085 Reporting");

  // begin I2C communication
  if (!bno08x.begin_I2C()) {
    Serial.println("Failed to find BNO085 chip");
    while (true) delay(10);
  }
  //Serial.println("BNO085 Found!");

  // Set report type
  if (!bno08x.enableReport(reportType, reportIntervalUs)) {
    Serial.println("Could not enable stabilized remote vector");
  }

  //Serial.println("Reading Events:");
  delay(100);
}

void loop() {
  // Check for resets and reset report types
  if (bno08x.wasReset()) {
    Serial.println("Sensor was reset.");
    if (!bno08x.enableReport(reportType, reportIntervalUs)) {
      Serial.println("Could not enable stabilized remote vector");
    }
  }


  // Check for sensor events
  if (bno08x.getSensorEvent(&sensorValue)) {
    x = sensorValue.un.arvrStabilizedRV.i;
    y = sensorValue.un.arvrStabilizedRV.j;
    z = sensorValue.un.arvrStabilizedRV.k;
    w = sensorValue.un.arvrStabilizedRV.real;
  }

  // Transmit data
  transmit();
}

void transmit() {
  Serial.print("{");
  Serial.print("\"x\":");
  Serial.print(x);
  Serial.print(",\"y\":");
  Serial.print(y);
  Serial.print(",\"z\":");
  Serial.print(z);
  Serial.print(",\"w\":");
  Serial.print(w);
  Serial.print(",\"time\":");
  Serial.print(millis());
  Serial.println("}");
}
