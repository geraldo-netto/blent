#include <linux/videodev2.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

static void layout(void)
{
    printf("%zu %zu %zu %zu %zu %zu %zu %zu %zu %zu %lu %lu %lu %lu %u %u %u %u\n",
           sizeof(struct v4l2_format), _Alignof(struct v4l2_format),
           offsetof(struct v4l2_format, fmt), sizeof(struct v4l2_pix_format),
           offsetof(struct v4l2_pix_format, sizeimage), sizeof(struct v4l2_streamparm),
           _Alignof(struct v4l2_streamparm), offsetof(struct v4l2_streamparm, parm),
           sizeof(struct v4l2_outputparm), sizeof(struct v4l2_fract), (unsigned long)VIDIOC_S_FMT,
           (unsigned long)VIDIOC_G_FMT, (unsigned long)VIDIOC_G_PARM, (unsigned long)VIDIOC_S_PARM,
           V4L2_PIX_FMT_YUV420, V4L2_FIELD_NONE, V4L2_BUF_TYPE_VIDEO_OUTPUT, V4L2_CAP_TIMEPERFRAME);
}

static int snapshot(void)
{
    struct v4l2_format format;
    struct v4l2_streamparm parameters;
    memset(&format, 0, sizeof(format));
    memset(&parameters, 0, sizeof(parameters));
    format.type = V4L2_BUF_TYPE_VIDEO_OUTPUT;
    parameters.type = V4L2_BUF_TYPE_VIDEO_OUTPUT;
    if (ioctl(STDIN_FILENO, VIDIOC_G_FMT, &format) < 0 ||
        ioctl(STDIN_FILENO, VIDIOC_G_PARM, &parameters) < 0)
    {
        perror("T1143 native snapshot");
        return 1;
    }
    printf("%u %u %u %u %u %u %u %u\n", format.fmt.pix.width, format.fmt.pix.height,
           format.fmt.pix.pixelformat, format.fmt.pix.field, format.fmt.pix.bytesperline,
           format.fmt.pix.sizeimage, parameters.parm.output.timeperframe.numerator,
           parameters.parm.output.timeperframe.denominator);
    return 0;
}

int main(int argc, char **argv)
{
    if (argc == 1)
    {
        layout();
        return 0;
    }
    if (argc == 2 && strcmp(argv[1], "--snapshot") == 0)
    {
        return snapshot();
    }
    return 2;
}
