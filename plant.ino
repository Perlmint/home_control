#include <WiFiNINA.h>
#include <WiFiServer.h>
#include <string>

char WIFI_SSID[] = "";
char WIFI_PASSWORD[] = "";
const int WIFI_KEY_INDEX = 0;

constexpr int MAX_LED_POWER = 255;
constexpr size_t LED_COUNT = 1;
constexpr int LED_INDEXES[LED_COUNT] = {10};
struct State
{
    int led_powers[LED_COUNT] = {0};
    int wifi_status = WL_IDLE_STATUS;
} STATE;

WiFiServer server(80);

void printWifiStatus()
{
    Serial.print("SSID: ");
    Serial.println(WiFi.SSID());

    IPAddress ip = WiFi.localIP();
    Serial.print("IP Address: ");
    Serial.println(ip);
    
    long rssi = WiFi.RSSI();
    Serial.print("signal strength (RSSI):");
    Serial.print(rssi);
    Serial.println(" dBm");
}

void setup()
{
    // set all led power off
    for (auto &power : STATE.led_powers)
    {
        power = 0;
    }
    for (size_t idx = 0; idx < LED_COUNT; idx++)
    {
        analogWrite(LED_INDEXES[idx], STATE.led_powers[idx]);
    }

    Serial.begin(115200);

    WiFi.setHostname("PlantLEDs");

    if (WiFi.status() == WL_NO_MODULE)
    {
        // don't continue
        while (true)
        {
            Serial.println("Communication with WiFi module failed!");
            delay(10000);
        }
    }

    String fv = WiFi.firmwareVersion();

    if (fv != WIFI_FIRMWARE_LATEST_VERSION)
    {
        Serial.println("Please upgrade the firmware");
    }

    // attempt to connect to Wifi network:
    while (STATE.wifi_status != WL_CONNECTED)
    {
        Serial.print("Attempting to connect to SSID: ");
        Serial.println(WIFI_SSID);
        // Connect to WPA/WPA2 network. Change this line if using open or WEP network:
        STATE.wifi_status = WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
        // wait 10 seconds for connection:
        delay(10000);
    }

    server.begin();

    // you're connected now, so print out the status:

    printWifiStatus();
}

void loop()
{
    WiFiClient client = server.available();

    if (client)
    {
        bool currentLineIsBlank = true;
        int consecutive_blank_lines = 0;
        std::string line = "";
        std::string body = "";
        bool header_end = false;

        std::string method = "";
        std::string uri = "";
        size_t content_length = 0;

        while (client.connected())
        {
            if (!client.available())
            {
                continue;
            }
            char c = client.read();
            Serial.print(c);

            if (!header_end) {
                line.push_back(c);

                if (c == '\n') {
                    if (currentLineIsBlank) {
                        header_end = true;
                        if (content_length == 0) {
                            break;
                        } else {
                            continue;
                        }
                    } else {
                        // you're starting a new line
                        if (method.empty()) {
                            const auto method_end = line.find(' ');
                            method = line.substr(0, method_end);
                            const auto uri_end = line.find_last_of(' ');
                            uri = line.substr(method_end + 1, uri_end - method_end - 1);
                        } else {
                            // header
                            const auto key_end = line.find(": ");
                            // to lowercase
                            for (size_t i = 0; i < key_end; i++) {
                                line[i] |= 0x20;
                            }
                            if (strncmp(line.c_str(), "content-length", key_end) == 0) {
                                content_length = atoll(line.c_str() + key_end + 1);
                            }
                        }

                        line.clear();
                    }
                    currentLineIsBlank = true;
                }
                else if (c != '\r')
                {
                    // you've gotten a character on the current line
                    currentLineIsBlank = false;
                }
            } else if (content_length > 0) {
                content_length--;
                body.push_back(c);

                if (content_length == 0) {
                    break;
                }
            }
        }

        if (header_end && content_length == 0) {
            Serial.println("--------- request end -------");

            size_t light_idx = 9999;
            std::string response;
            const char *http_status = "200 OK";
            Serial.println(method.data());
            Serial.println(uri.data());
            if (uri == "/lights/0/power") {
                // TODO: parse light index
                light_idx = 0;
                if (light_idx >= LED_COUNT) {
                    response = "Specified Light doesn't exist";
                    http_status = "404 NOTFOUND";
                } else {
                    if (method == "GET") {
                        response = std::to_string(STATE.led_powers[light_idx]);
                    } else if (method == "PUT") {
                        int power = std::atoi(body.data());
                        if (power > MAX_LED_POWER) {
                            response = "Too large power";
                            http_status = "400 BAD REQUEST";
                        } else {
                            STATE.led_powers[light_idx] = power;
                            response = std::to_string(power);
                        }
                    }
                }
            } else {
                response = "Unknown URI";
                http_status = "400 BAD REQUEST";
            }

            client.print("HTTP/1.1 ");
            client.println(http_status);
            client.print("Content-Length: ");
            client.println(response.length());
            client.println("Content-Type: text/plain");
            client.println("Connection: close"); // the connection will be closed after completion of the response
            client.println();
            client.println(response.c_str());
        }
    }

    // give the web browser time to receive the data
    delay(1);

    // close the connection:
    client.stop();

    for (size_t idx = 0; idx < LED_COUNT; idx++)
    {
        analogWrite(LED_INDEXES[idx], STATE.led_powers[idx]);
    }
}