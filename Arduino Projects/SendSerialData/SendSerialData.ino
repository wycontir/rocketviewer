#include <quaternion_type.h>

//placeholder for actual telemetry data, will be edited each loop to emulate new data
quat_t q1 = { 0, 1, 0, 0 };
float i = 1.0;

void setup() {
  //open serial port at 9600 baud
  Serial.begin(9600);
}


void loop() {
  //edit quat then transmit
  rotate_quat();
  transmit();
  i += 1;
  delay(10);
}

void rotate_quat() {
  //angle to rotate by
  float angle = (PI/180.0) * i;
  //axis
  vec3_t axis = { 0, 1, 0 };

  //rotate quat
  q1.setRotation(axis, angle, LARGE_ANGLE);


}



/*
current json output:
{
  "x": (quat x)
  "y": (quat y)
  "z": (quat z)
  "w": (quat w)
  "time": time in millis
}
*/

void transmit() {
  Serial.print("{");
  transmit_quat(q1);
  Serial.print("\"time\":");
  Serial.print(millis());
  Serial.println("}");
}

void transmit_quat(quat_t quat) {
  Serial.print("\"x\":");
  Serial.print(quat.v.x);
  Serial.print(",\"y\":");
  Serial.print(quat.v.y);
  Serial.print(",\"z\":");
  Serial.print(quat.v.z);
  Serial.print(",\"w\":");
  Serial.print(quat.w);
  Serial.print(",");
}