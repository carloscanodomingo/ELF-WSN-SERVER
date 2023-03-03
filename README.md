# ELF-WSN-SERVER
Server application written in **Rust** focuses on integrating with **InfluxDB2.0** to manage high datarate packets from multiples **IoT Magnetic sensors CC3220SF**.

## Why Rust and TOKIO?
### Definition
**Rust's** focus on performance, memory safety, concurrency, cross-platform support, and growing ecosystem make it an excellent choice for building IoT high data rate servers

### Advantages of Rust in this project
**Rust** is an ideal language for our project due to its key benefits of memory safety and concurrency. Since our project involves receiving data from multiple IoT devices simultaneously, ensuring memory safety can be challenging, especially when using a non-specific language. However, Rust provides a native solution to effectively manage this problem with its ownership memory system, which ensures that memory is handled correctly and avoids common issues like null pointer errors and memory leaks. Additionally, Rust's strong focus on performance ensures that our system runs smoothly, even under heavy loads. Rust's open-source packages also provide us with a wide range of options to implement the system, making it an excellent choice for our project.

1. **Performance**: Rust is a systems programming language designed from the ground up to prioritize performance and efficiency. It offers low-level control over system resources and has a highly optimized memory management system. These features make Rust well-suited for building high-performance servers that can handle large amounts of data at high rates.
2. **Memory Safety**: Rust's unique ownership and borrowing system ensures that memory safety is enforced at compile time, which reduces the likelihood of runtime crashes and security vulnerabilities. This is especially important for IoT devices, where security and reliability are critical.
3. **Concurrency**: Rust has built-in support for concurrency and parallelism, which enables it to handle multiple requests and connections simultaneously efficiently. Rust's concurrency features include asynchronous programming using futures and async/await and thread-safe communication using channels and mutexes.
4. **Cross-Platform Support**: Rust is a cross-platform language that can be compiled to run on various devices and operating systems, including embedded systems and IoT devices. This flexibility makes Rust an attractive option for building servers running on multiple hardware and software platforms.
5. **Growing Ecosystem**: Rust has a growing and vibrant ecosystem with a large and active community of developers and contributors. This means that many libraries, frameworks, and tools are available to help you build your IoT server quickly and efficiently. Rust's package manager, Cargo, also makes it easy to manage dependencies and produce and publish your own packages.

## Why InfluxDB2.0?
### Definition
InfluxDB is a high-performance, time-series database optimized for storing and querying IoT data. It offers a SQL-like query language, integrations with other tools, scalability, and is open source.

### Advantages of InfluxDB2.0 in this project
InfluxDB offers us the capability of managing large amounts of data collected by the high date-rate EM IoT sensor and arranging them in a single database with microsecond precision and high compression rate. 
InfluxDB provides an easy-to-use web server service to visualize and manage all the IoT data, including various techniques to process the data before the visualization, such as intermediate window representation.
With InfluxDB, artificial intelligence techniques will automatically monitor the state of all devices and extract frequency features of a regular collection of samples, providing valuable insights into IoT data.

### Characteristics
1. **Time-Series Database**: InfluxDB is a time-series database optimized for storing and querying data associated with a timestamp. This is ideal for IoT applications where data is generated continuously over time, and you need to store and analyze this data efficiently.
2. **High Write Performance**: InfluxDB is designed for high write performance, which means it can handle large volumes of incoming data at high rates. This makes it well-suited for IoT applications where you must process and store data in real time.

4. **Query Language**: InfluxDB uses a SQL-like query language called InfluxQL, explicitly designed for time-series data. This makes writing complex queries that retrieve data across different time ranges and perform calculations and aggregations easy.
5. **Integrations**: InfluxDB has integrations with a wide range of other tools and technologies, including popular data analysis and visualization tools like Grafana and Tableau. This makes it easy to build a complete data pipeline for your IoT application, from data collection to visualization and analysis.
6. **Scalability**: InfluxDB is designed to be highly scalable, with support for clustering and sharding. This means you can quickly scale your InfluxDB installation to handle increasing data and traffic as your IoT application grows.
7. **Open Source**: InfluxDB is open-source software, meaning it's free to use and modify. This makes it an attractive option for developers who want to build custom solutions and extend the functionality of the database.
8. 
## Why MQTT?
### Definition
MQTT is a lightweight and efficient messaging protocol used in IoT environments. It enables devices to send and receive messages with low overhead, making it well-suited for constrained networks and devices. **MQTT** uses a publish-subscribe model, where clients can publish letters to a broker, and other clients can subscribe to topics and receive messages from the broker.

### Advantages of MQTT in this project
In our IoT project, many packets are sent through a cost-effective device with significant constraints on power consumption due to battery and PV panel limitations. The wireless communication protocol is the most power-consuming service in the IoT field, with a significant portion dedicated to transmitting protocol-specific information rather than data. This is where **MQTT** comes in - it provides a simple protocol that can be implemented in low-performance devices and has a lightweight design, where almost 100% of the communication is dedicated to transmitting the payload. By using **MQTT**, we can significantly reduce the power consumption of our devices and achieve longer battery life or better utilization of the available power. Additionally, **MQTT's** publish-subscribe model allows for efficient communication between devices, making it an ideal choice for our IoT project
### Characteristics
1. **Lightweight design**: MQTT is a lightweight messaging protocol for resource-constrained environments. It's optimized for low-bandwidth, high-latency networks, making it an excellent choice for managing high data rate packets from several IoT devices.
2. **Publish-subscribe messaging pattern**: MQTT uses a publish-subscribe messaging way, where data is published to a broker and then distributed to one or more subscribers. This allows multiple devices to send data to a central broker, which can be processed, analyzed, and stored in real time.
3. **Low overhead**: One of the critical advantages of MQTT is its low overhead, which means it can handle many small messages with minimal network traffic. This makes it ideal for IoT applications that generate a high volume of data, as it can efficiently manage and transmit large amounts of data with minimal impact on network bandwidth.
4. **Quality of Service (QoS) levels**: MQTT supports QoS levels, which provide different levels of reliability and delivery guarantees for messages. This allows the protocol to prioritize critical data and ensure that it is delivered in a timely and reliable manner.
5. **Scalability**: MQTT is highly scalable, supporting many devices and brokers. This makes it an ideal protocol for IoT applications, where the number of connected devices can rapidly grow over time.
6. **Security**: MQTT supports various security mechanisms, including TLS encryption and username/password authentication, making it a secure option for transmitting sensitive IoT data.


## Why CC3220SF?
### Definition
The Texas Instruments **CC3200SF** device is a System-on-Chip (SoC) microcontroller designed for Internet of Things (IoT) applications. It features a powerful ARM Cortex-M4 processor with built-in Wi-Fi connectivity, making connecting to the internet and other devices easy. The **CC3200SF** also includes various integrated peripherals such as UART, I2C, and SPI interfaces, making it a versatile and flexible platform for many IoT applications. The device is also designed with low power consumption in mind, making it an ideal choice for battery-powered IoT devices.
### Advantages of CC3200SF in this project
Our IoT project requires a device that can handle high data rates through ADC devices while being power-efficient and cost-effective for scalability. The CC3200SF is an ideal choice for these requirements, offering a 12-bit ADC for capturing high data rates while consuming low power in its sleep mode. Moreover, the device incorporates a Wi-Fi network processor, a microcontroller, and a high-performance ARM Cortex-M4 MCU for efficient data processing. Texas Instruments Real-Time Operating System TIRTOS and SDK enhance the device's programmability, reduces development and debug time, and simplify integration with other devices in our IoT network. The **CC3200SF** provides us with the technical features and resources to build a robust and scalable IoT solution.
### Characteristics
1. High-Speed ADC data collection: The Texas Instruments **CC3200SF** device features a powerful ARM Cortex-M4 processor that can efficiently collect and process large amounts of ADC data at high speeds. This makes it an excellent choice for applications that require the collection of high-speed analog data, such as industrial monitoring, control systems, or scientific data acquisition.
2. Built-in Wi-Fi Connectivity: The **CC3200SF** has built-in Wi-Fi connectivity, making establishing a wireless connection to other devices or servers easy. This is essential for IoT applications where data needs to be transmitted wirelessly over the internet.
3. MQTT Protocol Support: MQTT is a lightweight and efficient messaging protocol widely used in IoT applications. It is designed to handle large amounts of data and is highly optimized for low bandwidth and power consumption. The **CC3200SF** device supports MQTT, making it an excellent choice for applications that require the efficient transmission of large amounts of data over the internet.
4. Low Power Consumption: The **CC3200SF** device is designed with low power consumption in mind, making it ideal for battery-powered or remote IoT applications. It features several power-saving modes, including a low-power sleep mode, which allows the device to conserve energy when it is not in use.
5. Easy to Implement: The **CC3200SF** device is easy to implement, thanks to its integrated peripherals such as UART, I2C, and SPI interfaces. These interfaces allow for easy integration with other sensors and devices, making it an excellent choice for IoT applications requiring data collection and transmission from multiple sources.


