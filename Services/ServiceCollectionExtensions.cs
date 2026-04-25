using System;
using FlowReveal.Core.Interfaces;
using Microsoft.Extensions.DependencyInjection;

namespace FlowReveal.Services
{
    public static class ServiceCollectionExtensions
    {
        public static IServiceCollection AddFlowRevealServices(this IServiceCollection services)
        {
            services.AddSingleton<IProtocolParser, Parser.ProtocolParser>();
            services.AddSingleton<IFilterEngine, Filter.FilterEngine>();
            services.AddSingleton<ISessionStore, Session.SessionStore>();
            return services;
        }

        public static IServiceCollection AddWindowsPlatformServices(this IServiceCollection services)
        {
            services.AddSingleton<IPacketCaptureService, Platforms.Windows.Capture.WindowsPacketCaptureService>();
            return services;
        }
    }
}
