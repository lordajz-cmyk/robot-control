/*
    nodstopp.cpp — se nodstopp.h.
*/

#include "nodstopp.h"

#include <QDir>
#include <cstring>
#include <fcntl.h>
#include <linux/gpio.h>
#include <sys/ioctl.h>
#include <unistd.h>

NodstoppGpio::~NodstoppGpio()
{
    if (mLineFd >= 0) {
        ::close(mLineFd);
    }
}

bool NodstoppGpio::open(int gpio, QString &fel)
{
    const QByteArray namn = QString("GPIO%1").arg(gpio).toLatin1();
    const QStringList chips = QDir("/dev").entryList(QStringList() << "gpiochip*", QDir::System);

    for (const QString &chip: chips) {
        int fd = ::open(QString("/dev/" + chip).toLatin1().constData(), O_RDWR | O_CLOEXEC);
        if (fd < 0) {
            continue;
        }

        struct gpiochip_info info;
        memset(&info, 0, sizeof(info));
        if (ioctl(fd, GPIO_GET_CHIPINFO_IOCTL, &info) < 0) {
            ::close(fd);
            continue;
        }

        for (unsigned int i = 0;i < info.lines;i++) {
            struct gpio_v2_line_info li;
            memset(&li, 0, sizeof(li));
            li.offset = i;
            if (ioctl(fd, GPIO_V2_GET_LINEINFO_IOCTL, &li) < 0 || strcmp(li.name, namn.constData()) != 0) {
                continue;
            }

            struct gpio_v2_line_request req;
            memset(&req, 0, sizeof(req));
            req.offsets[0] = i;
            req.num_lines = 1;
            strncpy(req.consumer, "car_client-nodstopp", sizeof(req.consumer) - 1);
            req.config.flags = GPIO_V2_LINE_FLAG_INPUT | GPIO_V2_LINE_FLAG_BIAS_PULL_UP;

            if (ioctl(fd, GPIO_V2_GET_LINE_IOCTL, &req) < 0) {
                fel = QString("kunde inte ta %1 på /dev/%2: %3 (används den av något annat?)")
                        .arg(QString(namn), chip, QString(strerror(errno)));
                ::close(fd);
                return false;
            }

            ::close(fd);
            mLineFd = req.fd;
            return true;
        }

        ::close(fd);
    }

    fel = QString("hittade ingen %1 i /dev/gpiochip* (saknas behörighet? användaren ska vara med i gruppen gpio)")
            .arg(QString(namn));
    return false;
}

int NodstoppGpio::lasSluten()
{
    if (mLineFd < 0) {
        return -1;
    }

    struct gpio_v2_line_values v;
    memset(&v, 0, sizeof(v));
    v.mask = 1;
    if (ioctl(mLineFd, GPIO_V2_LINE_GET_VALUES_IOCTL, &v) < 0) {
        return -1;
    }

    // Låg = kontakten sluten mot GND = OK.
    return (v.bits & 1) ? 0 : 1;
}
