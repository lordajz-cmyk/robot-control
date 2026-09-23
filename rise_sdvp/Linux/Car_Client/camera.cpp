/*
    Copyright 2019 Benjamin Vedder	benjamin@vedder.se

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

#include "camera.h"
#include "utility.h"

#include <QDebug>
#include <QMediaDevices>
#include <QCameraDevice>
#include <QMediaCaptureSession>
//#include <QCameraInfo>
#include <QFile>
#include <QCameraFormat>
//#include <QCameraViewfinderSettings>
#include <QTime>

//MyVideoSurface::MyVideoSurface(QObject *parent) : QAbstractVideoSurface(parent)
MyVideoSurface::MyVideoSurface(QObject *parent) : QVideoSink(parent)
{

}

//QList<QVideoFrame::PixelFormat> MyVideoSurface::supportedPixelFormats(QAbstractVideoBuffer::HandleType) const
QList<QVideoFrameFormat::PixelFormat> MyVideoSurface::supportedPixelFormats(QVideoFrame::HandleType) const
{
    QList<QVideoFrameFormat::PixelFormat> result;
    result  << QVideoFrameFormat::Format_RGBX8888;
//    result  << QVideoFrame::Format_RGB32 << QVideoFrame::Format_RGB24;
    return result;
}

bool MyVideoSurface::present(const QVideoFrame &frame)
{
    QVideoFrame localCopy = frame;

    localCopy.map(QVideoFrame::ReadOnly);
//    localCopy.map(QAbstractVideoBuffer::ReadOnly);

    QImage img(localCopy.bits(0), localCopy.width(), localCopy.height(),
               QVideoFrameFormat::imageFormatFromPixelFormat(localCopy.pixelFormat()));

//    QImage img(localCopy.bits(), localCopy.width(), localCopy.height(),
//               QVideoFrame::imageFormatFromPixelFormat(localCopy.pixelFormat()));

    emit imageCaptured(img);

//    img.save(QString("/home/benjamin/Skrivbord/images/test%1.jpg").
//             arg(QTime::currentTime().msecsSinceStartOfDay()),
//             0, -1);

    localCopy.unmap();

    return true;
}

Camera::Camera(QObject *parent) : QObject(parent)
{
    mCamera = 0;
    mVid = new MyVideoSurface(this);
}

MyVideoSurface *Camera::video()
{
    return mVid;
}

bool Camera::openCamera(int cameraIndex)
{
    int res = false;

//    QList<QCameraDevice> availableCameras = QCameraDevice::availableCameras();
    const QList<QCameraDevice> availableCameras = QMediaDevices::videoInputs();

    for (const QCameraDevice &cameraInfo : availableCameras)

    if (availableCameras.size() > cameraIndex) {
        mCamera = new QCamera(availableCameras.at(cameraIndex), this);
//        mCamera->setViewfinder(mVid);
//        mCamera->setCaptureMode(QCamera::CaptureViewfinder);
//        mCamera->load();

        QMediaCaptureSession mediaCaptureSession;
//        mediaCaptureSession.setCamera(&mCamera);
        mediaCaptureSession.setCamera(mCamera);
//        mediaCaptureSession.setVideoOutput(&mVid);
        mediaCaptureSession.setVideoOutput(mVid);

        utility::waitSignal(mCamera, SIGNAL(stateChanged(QCamera::State)), 1000);
        res = true;
    } else {
        qWarning() << "Camera not found";
    }

    return res;
}

void Camera::closeCamera()
{
    if (mCamera) {
        mCamera->stop();
        delete mCamera;
        mCamera = 0;
    }
}

bool Camera::startCameraStream(int width, int height, int fps)
{
    int res = false;

    if (mCamera) {
        /*
        QCameraFormat vfSet = mCamera->viewfinderSettings();
        //QCameraViewfinderSettings vfSet = mCamera->viewfinderSettings();

        if (fps > 0) {
            vfSet.setMinimumFrameRate(fps);
            vfSet.setMaximumFrameRate(fps);
        }

        if (width > 0 && height > 0) {
            vfSet.setResolution(width, height);
        }
        */




//        auto a = mCamera->supportedViewfinderPixelFormats();
//        if (a.contains(QVideoFrame::Format_RGB24)) {
//            vfSet.setPixelFormat(QVideoFrame::Format_RGB24);
//        } else if (a.contains(QVideoFrame::Format_RGB32)) {
//            vfSet.setPixelFormat(QVideoFrame::Format_RGB32);
//        }

//        vfSet.setPixelFormat(QVideoFrame::Format_Jpeg);
//        vfSet.setPixelFormat(QVideoFrame::Format_YUYV);

        // mCamera->setViewfinderSettings(vfSet);
        mCamera->start();

        res = true;
        qWarning() << "Camera open";
    } else {
        qWarning() << "No camera open";
    }

    return res;
}

QString Camera::cameraInfo()
{
    QString res;
    QTextStream out(&res);
/*
    if (mCamera) {
        res += "Locks supported: " + QString::number(mCamera->supportedLocks(), 16) + "\n";

        for (auto a: mCamera->supportedViewfinderPixelFormats()) {
            res += "VF PixelFormat: " + QString::number(a) + "\n";
        }

        for (auto a: mCamera->supportedViewfinderFrameRateRanges()) {
            res += "VF Rate: " + QString::number(a.minimumFrameRate) +
                    " " + QString::number(a.maximumFrameRate) + "\n";
        }

        for (auto a: mCamera->supportedViewfinderResolutions()) {
            res += "VF Res: " + QString::number(a.width()) + "x" +
                    QString::number(a.height()) + "\n";
        }

        if (res.endsWith("\n")) {
            res.chop(1);
        }
    }
*/
    const QList<QCameraFormat> videoFormats = mCamera->cameraDevice().videoFormats();

    for (const QCameraFormat &format : videoFormats)
    {
        out << QVideoFrameFormat::pixelFormatToString(format.pixelFormat())
                 << " (" << format.resolution().width()
                 << ")x(" << format.resolution().height()
                 << ") " << format.minFrameRate()
                 << '-' << format.maxFrameRate() << "FPS";
    }

    return res;
}

bool Camera::isLoaded()
{
    bool res = false;

    if (mCamera) {
/*        if (mCamera->state() == QCamera::LoadedState ||
                mCamera->status() == QCamera::ActiveStatus) {
            res = true;
        }
*/
        if (mCamera->isActive())
        {
            res=true;
        }
    }

    return res;
}
