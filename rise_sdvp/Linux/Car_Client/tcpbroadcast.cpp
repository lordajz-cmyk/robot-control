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

#include "tcpbroadcast.h"

/**
 * Constructor for TcpBroadcast.
 * Initializes TCP server and sets up signal-slot connections.
 * 
 * @param parent Parent QObject
 */
TcpBroadcast::TcpBroadcast(QObject *parent) : QObject(parent)
{
    // Create TCP server
    mTcpServer = new QTcpServer(this);
    
    // Connect new connection signal to slot
    connect(mTcpServer, SIGNAL(newConnection()), this, SLOT(newTcpConnection()));
}

/**
 * Destructor for TcpBroadcast.
 * Stops logging before destruction.
 */
TcpBroadcast::~TcpBroadcast()
{
    logStop();
}

/**
 * Start TCP server on specified port.
 * 
 * @param port TCP port to listen on
 * @return true if server started successfully, false otherwise
 */
bool TcpBroadcast::startTcpServer(int port)
{
    // Start listening on specified port
    if (!mTcpServer->listen(QHostAddress::Any,  port)) {
        qWarning() << "Unable to start TCP server: " << mTcpServer->errorString();
        return false;
    }

    return true;
}

/**
 * Get the last error that occurred.
 * 
 * @return Error string describing the last error
 */
QString TcpBroadcast::getLastError()
{
    return mTcpServer->errorString();
}

/**
 * Stop the TCP server and clean up all connections.
 * Closes the server and deletes all connected sockets.
 */
void TcpBroadcast::stopServer()
{
    // Close the server
    mTcpServer->close();
    
    // Clean up all connected sockets
    QMutableListIterator<QTcpSocket*> itr(mSockets);

    while(itr.hasNext()) {
        QTcpSocket *socket = itr.next();
        socket->deleteLater();
        itr.remove();
    }
}

void TcpBroadcast::broadcastData(QByteArray data)
{
    QMutableListIterator<QTcpSocket*> itr(mSockets);

    if (mLog.isOpen()) {
        mLog.write(data);
    }

    while(itr.hasNext()) {
        QTcpSocket *socket = itr.next();
        if (socket->isOpen()) {
            socket->write(data);
        } else {
            socket->deleteLater();
            itr.remove();
        }
    }
}

bool TcpBroadcast::logToFile(QString file)
{
    if (mLog.isOpen()) {
        mLog.close();
    }

    mLog.setFileName(file);
    return mLog.open(QIODevice::ReadWrite | QIODevice::Truncate);
}

void TcpBroadcast::logStop()
{
    if (mLog.isOpen()) {
        qDebug() << "Closing log:" << mLog.fileName();
        mLog.close();
    }
}

void TcpBroadcast::newTcpConnection()
{
    mSockets.append(mTcpServer->nextPendingConnection());
    connect(mSockets.last(), SIGNAL(readyRead()),
            this, SLOT(readyRead()));
    qDebug() << "TCP connection accepted:" << mSockets[mSockets.size() - 1]->peerAddress();
}

void TcpBroadcast::readyRead()
{
    QMutableListIterator<QTcpSocket*> itr(mSockets);

    while(itr.hasNext()) {
        QTcpSocket *socket = itr.next();
        QByteArray data = socket->readAll();
        if (!data.isEmpty()) {
            emit dataReceived(data);
        }
    }
}

