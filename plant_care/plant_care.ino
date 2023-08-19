#if defined(ESP8266)
#include <ESP8266WiFi.h>
#include <ArduinoOTA.h>
#define USE_OTA
#elif defined(ARDUINO_SAMD_NANO_33_IOT)
#include <WiFiNINA.h>
#define USE_WIFI_NINA
#endif
#include <WiFiServer.h>
#include <string>

const char WIFI_SSID[] = "";
const char WIFI_PASSWORD[] = "";
const int WIFI_KEY_INDEX = 0;

constexpr int MAX_OUT_POWER = 255;
constexpr size_t OUT_COUNT = 2;
constexpr int OUT_INDEXES[OUT_COUNT] = {
    14,
    12};
struct State
{
    int out_powers[OUT_COUNT] = {0};
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

void connectWifi()
{
    int wifi_status = WL_IDLE_STATUS;
    while (wifi_status != WL_CONNECTED)
    {
        Serial.print("Attempting to connect to SSID: ");
        Serial.println(WIFI_SSID);
        // Connect to WPA/WPA2 network. Change this line if using open or WEP network:
        wifi_status = WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
        // wait 10 seconds for connection:
        for (int i = 0; i < 10; i++)
        {
            yield();
            delay(1000);
        }
        wifi_status = WiFi.status();
    }

    server.begin();

    // you're connected now, so print out the status:
    printWifiStatus();
}

void setup()
{
    Serial.begin(115200);

    // set all led power off
    for (auto &power : STATE.out_powers)
    {
        power = 0;
    }
    for (size_t idx = 0; idx < OUT_COUNT; idx++)
    {
        pinMode(OUT_INDEXES[idx], OUTPUT);
        analogWriteRange(MAX_OUT_POWER);
        analogWrite(OUT_INDEXES[idx], STATE.out_powers[idx]);
    }

    WiFi.setHostname("PlantOUTs");

#if defined(USE_WIFI_NINA)
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
#endif

#if defined(USE_OTA)
    ArduinoOTA.begin();
#endif
}

void loop()
{
    if (WiFi.status() != WL_CONNECTED)
    {
        connectWifi();
        yield();
    }

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
            yield();

            if (!header_end)
            {
                line.push_back(c);

                if (c == '\n')
                {
                    if (currentLineIsBlank)
                    {
                        header_end = true;
                        if (content_length == 0)
                        {
                            break;
                        }
                        else
                        {
                            continue;
                        }
                    }
                    else
                    {
                        // you're starting a new line
                        if (method.empty())
                        {
                            const auto method_end = line.find(' ');
                            method = line.substr(0, method_end);
                            const auto uri_end = line.find_last_of(' ');
                            uri = line.substr(method_end + 1, uri_end - method_end - 1);
                        }
                        else
                        {
                            // header
                            const auto key_end = line.find(": ");
                            // to lowercase
                            for (size_t i = 0; i < key_end; i++)
                            {
                                line[i] |= 0x20;
                            }
                            if (strncmp(line.c_str(), "content-length", key_end) == 0)
                            {
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
            }
            else if (content_length > 0)
            {
                content_length--;
                body.push_back(c);

                if (content_length == 0)
                {
                    break;
                }
            }
        }

        if (header_end && content_length == 0)
        {
            Serial.println("--------- request end -------");

            size_t light_idx = 9999;
            std::string response;
            const char *http_status = "200 OK";
            Serial.println(method.data());
            Serial.println(uri.data());

            size_t prev_pos = 1;
            size_t idx = 0;
            int out_idx = -1;
            bool stop = false;
            while (!stop)
            {
                size_t pos = uri.find('/', prev_pos);
                if (pos == -1)
                {
                    pos = uri.length() - 1;
                    stop = true;
                }
                else
                {
                    uri[pos] = '\0';
                }
                const char *fragment = uri.data() + prev_pos;
                Serial.printf("fragment(%d[%d-%d]): ", idx, prev_pos, pos);
                Serial.println(fragment);
                switch (idx)
                {
                case 0:
                    if (strncmp(fragment, "power", 5) != 0)
                    {
                        stop = true;
                    }
                    break;
                case 1:
                    out_idx = atoi(fragment);
                    break;
                default:
                    out_idx = -1;
                    stop = true;
                    break;
                }
                idx++;
                prev_pos = pos + 1;
            }
            if (out_idx != -1)
            {
                if (out_idx >= OUT_COUNT)
                {
                    response = "Specified Out doesn't exist";
                    http_status = "404 NOTFOUND";
                }
                else
                {
                    if (method == "GET")
                    {
                        response = std::to_string(STATE.out_powers[out_idx]);
                    }
                    else if (method == "PUT")
                    {
                        int power = std::atoi(body.data());
                        if (power > MAX_OUT_POWER)
                        {
                            response = "Too large power";
                            http_status = "400 BAD REQUEST";
                        }
                        else
                        {
                            STATE.out_powers[out_idx] = power;
                            response = std::to_string(power);
                        }
                    }
                }
            }
            else
            {
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
            client.flush();
        }
        client.stop();
    }

#if defined(USE_OTA)
    ArduinoOTA.handle();
#endif

    for (size_t idx = 0; idx < OUT_COUNT; idx++)
    {
        analogWrite(OUT_INDEXES[idx], STATE.out_powers[idx]);
    }
}
