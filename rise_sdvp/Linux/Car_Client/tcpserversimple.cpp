/*
    Copyright 2016 Benjamin Vedder	benjamin@vedder.se

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
    */

#include "tcpserversimple.h"
#include <QDebug>
#include <QLocalSocket>

/**
 * Constructor for TcpServerSimple.
 * Initializes TCP server, packet handler, and sets up signal-slot connections.
 * 
 * @param parent Parent QObject
 */
TcpServerSimple::TcpServerSimple(QObject *parent) : QObject(parent)
{
    // Create TCP server
    mTcpServer = new QTcpServer(this);
    
    // Create packet handler
    mPacket = new Packet(this);
    
    // Initialize state
    mTcpSocket = 0;
    mUsePacket = false;

    // Connect signals
    connect(mTcpServer, SIGNAL(newConnection()), this, SLOT(newTcpConnection()));
    connect(mPacket, SIGNAL(dataToSend(QByteArray&)),
            this, SLOT(dataToSend(QByteArray&)));
}

/**
 * Start TCP server on specified port and address.
 * 
 * @param port TCP port to listen on
 * @param addr Network address to bind to (default: Any)
 * @return true if server started successfully, false otherwise
 */
bool TcpServerSimple::startServer(int port, QHostAddress addr)
{
	qDebug() << "in TcpServerSimple::startServer";
	qDebug() << "(adress: " << addr << ", port: " << port;
    
    // Start listening on specified address and port
    if (!mTcpServer->listen(addr,  port)) {
        return false;
    }
    return true;
}

/**
 * Stop the TCP server and clean up the connection.
 * Closes the server, notifies about disconnection, and deletes the socket.
 */
void TcpServerSimple::stopServer()
{
//	qDebug() << "in TcpServerSimple::stoptServer";
    // Close the server
    mTcpServer->close();

    // Clean up the client connection
    if (mTcpSocket) {
        // Notify about disconnection
        emit connectionChanged(false, mTcpSocket->peerAddress().toString());
        
        // Close and delete the socket
        mTcpSocket->close();
        delete mTcpSocket;
        mTcpSocket = 0;
    }
}

/**
 * Send data to the connected client.
 * Also forwards data to ROS 2 channel if available.
 * 
 * @param data Data to send
 * @return true if data was sent successfully, false if no client connected
 */
bool TcpServerSimple::sendData(const QByteArray &data)
{
//    qDebug() << "in TcpServerSimple::sendData: " << data;
    bool res = false;

    // Send data to connected client if available
    if (mTcpSocket) {
        mTcpSocket->write(data);
        sendMessageToRos2(data);
        res = true;
    }

    return res;
}

void TcpServerSimple::sendMessageToRos2(const QByteArray& data)
{
    QLocalSocket socket;
    socket.connectToServer("carclient_ros2_channel");

    if (!socket.waitForConnected(3000)) {
//        qDebug() << "Failed to connect to ROS2 channel";
        return;
    }
    qDebug() << "in TcpServerSimple::sendMessageToRos2: " << data;
    socket.write(data);
    socket.flush();
    socket.disconnectFromServer();
}

QString TcpServerSimple::errorString()
{
    return mTcpServer->errorString();
}

Packet *TcpServerSimple::packet()
{
    return mPacket;
}

void TcpServerSimple::newTcpConnection()
{
    QTcpSocket *socket = mTcpServer->nextPendingConnection();
    socket->setSocketOption(QAbstractSocket::LowDelayOption, true);

    if (mTcpSocket) {
        socket->close();
        delete socket;
    } else {
        mTcpSocket = socket;

        if (mTcpSocket) {
            connect(mTcpSocket, SIGNAL(readyRead()), this, SLOT(tcpInputDataAvailable()));
            connect(mTcpSocket, SIGNAL(disconnected()),
                    this, SLOT(tcpInputDisconnected()));
 /*           connect(mTcpSocket, SIGNAL(error(QAbstractSocket::SocketError)),
                    this, SLOT(tcpInputError(QAbstractSocket::SocketError)));*/
            emit connectionChanged(true, mTcpSocket->peerAddress().toString());
        }
    }
}

void TcpServerSimple::tcpInputDisconnected()
{
    emit connectionChanged(false, mTcpSocket->peerAddress().toString());
    mTcpSocket->deleteLater();
    mTcpSocket = 0;
}

void TcpServerSimple::tcpInputDataAvailable()
{
//	qDebug() << "in TcpServerSimple::tcpInputDataAvailable";
    QByteArray data = mTcpSocket->readAll();
    emit dataRx(data);

    if (mUsePacket) {
//        qDebug() << "use Packet";
        mPacket->processData(data);
    } else {
//    	qDebug() << "do not use Packet";
    }
}

void TcpServerSimple::tcpInputError(QAbstractSocket::SocketError socketError)
{
    (void)socketError;
    mTcpSocket->abort();
//    qDebug() << socketError;
}

void TcpServerSimple::dataToSend(QByteArray &data)
{
    sendData(data);
}

bool TcpServerSimple::usePacket() const
{
    return mUsePacket;
}

void TcpServerSimple::setUsePacket(bool usePacket)
{
    mUsePacket = usePacket;
}
